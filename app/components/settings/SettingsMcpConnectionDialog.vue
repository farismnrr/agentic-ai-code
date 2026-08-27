<script setup lang="ts">
import * as v from 'valibot'
import type { FormSubmitEvent } from '@nuxt/ui'
import type { McpOAuthDiscovery, McpRemoteConfig, McpRemoteTransport, McpScanResult, McpServer } from '#shared/types/chat'

const props = defineProps<{
  server?: McpServer | null
}>()

const emit = defineEmits<{
  saved: []
}>()

const open = defineModel<boolean>('open', { default: false })
const { discoverOAuth, startOAuth, scan, create, update } = useMcpServers()
const telemetry = useTelemetry()
const toast = useToast()

const schema = v.strictObject({
  name: v.pipe(v.string(), v.trim(), v.minLength(1, 'Name is required'), v.maxLength(80, 'Name is too long')),
  description: v.pipe(v.string(), v.trim(), v.maxLength(280, 'Description is too long')),
  transport: v.picklist(['http', 'sse'] as const),
  url: v.pipe(v.string(), v.trim(), v.minLength(1, 'URL is required'), v.maxLength(2048, 'URL is too long'), v.url('Enter a valid URL'))
})

type Schema = v.InferOutput<typeof schema>
type AuthChoice = 'auto' | 'oauth' | 'none'

const state = reactive<{ name: string, description: string, transport: McpRemoteTransport, url: string }>({
  name: '',
  description: '',
  transport: 'http',
  url: ''
})
const authChoice = ref<AuthChoice>('auto')
const acknowledgedRisk = ref(false)
const oauthDiscovery = ref<McpOAuthDiscovery | null>(null)
const discoveringAuth = ref(false)
const discoveryKey = ref<string | null>(null)
const advancedOpen = ref(false)
const scanResult = ref<McpScanResult | null>(null)
const scannedKey = ref<string | null>(null)
const errorMessage = ref<string | null>(null)
const scanning = ref(false)
const saving = ref(false)
const submitIntent = ref<'scan' | 'save'>('scan')
let discoveryTimer: ReturnType<typeof setTimeout> | undefined

const isEditing = computed(() => Boolean(props.server))
const currentConnectionKey = computed(() => `${state.transport}|${state.url.trim()}`)
const scanFresh = computed(() => Boolean(scanResult.value) && scannedKey.value === currentConnectionKey.value)
const connectionChanged = computed(() => {
  if (!props.server) return true
  return state.transport !== props.server.transport || state.url.trim() !== (props.server.url ?? '')
})
const hasChanges = computed(() => {
  if (!props.server) return true
  return state.name.trim() !== props.server.name
    || state.description.trim() !== props.server.description
    || connectionChanged.value
})
const effectiveAuth = computed<'oauth' | 'none'>(() => {
  if (authChoice.value === 'oauth') return 'oauth'
  if (authChoice.value === 'none') return 'none'
  return oauthDiscovery.value?.available ? 'oauth' : 'none'
})
const canCreate = computed(() => acknowledgedRisk.value && state.name.trim().length > 0 && state.url.trim().length > 0 && !saving.value && !scanning.value)
const canSave = computed(() => {
  if (!isEditing.value) return canCreate.value
  if (!hasChanges.value) return false
  return !connectionChanged.value || scanFresh.value
})
const dialogTitle = computed(() => props.server ? `Manage ${props.server.name}` : 'New MCP connection')
const authItems = [
  { label: 'Automatic (recommended)', value: 'auto' },
  { label: 'OAuth', value: 'oauth' },
  { label: 'No authentication', value: 'none' }
]

watch(currentConnectionKey, () => {
  if (scannedKey.value !== currentConnectionKey.value) {
    scanResult.value = null
  }
})

watch(() => state.url, (value) => {
  if (isEditing.value) return
  oauthDiscovery.value = null
  discoveryKey.value = null
  if (discoveryTimer) clearTimeout(discoveryTimer)

  const next = value.trim()
  if (!next || !/^https?:\/\//i.test(next)) return
  discoveryTimer = setTimeout(() => {
    void inspectAuthorization()
  }, 500)
})

watch(open, (value) => {
  if (!value) return
  resetDialog()
})

onBeforeUnmount(() => {
  if (discoveryTimer) clearTimeout(discoveryTimer)
})

function resetDialog() {
  if (discoveryTimer) clearTimeout(discoveryTimer)
  oauthDiscovery.value = null
  discoveryKey.value = null
  discoveringAuth.value = false
  authChoice.value = 'auto'
  acknowledgedRisk.value = false
  advancedOpen.value = false
  scanResult.value = null
  scannedKey.value = null
  errorMessage.value = null
  scanning.value = false
  saving.value = false
  submitIntent.value = 'scan'

  if (props.server) {
    Object.assign(state, {
      name: props.server.name,
      description: props.server.description,
      transport: props.server.transport === 'sse' ? 'sse' : 'http',
      url: props.server.url ?? ''
    })
    return
  }

  Object.assign(state, { name: '', description: '', transport: 'http', url: '' })
}

function remoteConfig(data: Schema): McpRemoteConfig {
  return {
    name: data.name,
    description: data.description,
    transport: data.transport,
    url: data.url
  }
}

async function inspectAuthorization() {
  const url = state.url.trim()
  if (!url || discoveryKey.value === url || discoveringAuth.value) return
  discoveringAuth.value = true
  errorMessage.value = null
  try {
    oauthDiscovery.value = await discoverOAuth(url)
    discoveryKey.value = url
    if (oauthDiscovery.value.available) advancedOpen.value = true
  } catch {
    oauthDiscovery.value = null
    discoveryKey.value = null
  } finally {
    discoveringAuth.value = false
  }
}

async function createConnection(config: McpRemoteConfig) {
  scanning.value = true
  try {
    const result = await scan(config)
    scanResult.value = result
    scannedKey.value = `${config.transport}|${config.url}`
  } finally {
    scanning.value = false
  }

  saving.value = true
  try {
    await create(config)
    toast.add({
      title: 'MCP connection created',
      description: `${scanResult.value?.tools.length ?? 0} tools discovered.`,
      icon: 'i-lucide-check',
      color: 'success'
    })
    emit('saved')
    open.value = false
  } finally {
    saving.value = false
  }
}

async function onSubmit(event: FormSubmitEvent<Schema>) {
  const config = remoteConfig(event.data)
  errorMessage.value = null

  if (!isEditing.value) {
    if (!canCreate.value) return
    try {
      if (authChoice.value === 'auto' && discoveryKey.value !== config.url) {
        oauthDiscovery.value = await discoverOAuth(config.url)
        discoveryKey.value = config.url
      }
      if (effectiveAuth.value === 'oauth') {
        saving.value = true
        telemetry.logEvent('info', 'mcp.oauth.start', 'MCP OAuth start requested', {
          'operation': 'mcp.oauth.start',
          'outcome': 'success',
          'mcp.stage': 'frontend_start',
          'mcp.transport': config.transport,
          'mcp.oauth': true
        })
        const result = await startOAuth(config)
        telemetry.logEvent('info', 'mcp.oauth.redirect', 'MCP OAuth authorization redirect ready', {
          'operation': 'mcp.oauth.redirect',
          'outcome': 'success',
          'mcp.stage': 'frontend_redirect',
          'mcp.transport': config.transport,
          'mcp.oauth': true
        })
        telemetry.flush()
        await navigateTo(result.authorizationUrl, { external: true })
        return
      }
      await createConnection(config)
    } catch (err: unknown) {
      if (effectiveAuth.value === 'oauth') {
        telemetry.logError('mcp.oauth.start', err, {
          'operation': 'mcp.oauth.start',
          'mcp.stage': 'frontend_start',
          'mcp.transport': config.transport,
          'mcp.oauth': true
        })
      }
      errorMessage.value = clientErrorMessage(
        err,
        effectiveAuth.value === 'oauth'
          ? 'Could not start OAuth sign-in. The authorization server may reject the AI Code callback URL.'
          : 'Could not connect. Check the server URL and access, then try again.'
      )
    } finally {
      saving.value = false
    }
    return
  }

  if (submitIntent.value === 'scan') {
    scanning.value = true
    try {
      const result = await scan(config)
      scanResult.value = result
      scannedKey.value = `${config.transport}|${config.url}`
    } catch (err: unknown) {
      scanResult.value = null
      scannedKey.value = null
      errorMessage.value = clientErrorMessage(err, 'Could not connect. Check the URL, transport, and server access, then scan again.')
    } finally {
      scanning.value = false
    }
    return
  }

  if (!canSave.value) return
  saving.value = true
  try {
    await update(props.server!.id, config)
    toast.add({ title: 'MCP connection updated', icon: 'i-lucide-check', color: 'success' })
    emit('saved')
    open.value = false
  } catch (err: unknown) {
    errorMessage.value = clientErrorMessage(err, 'Could not save this MCP connection. Recheck the server and try again.')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <UModal
    v-model:open="open"
    :title="dialogTitle"
    :description="isEditing ? 'Update and verify this remote MCP connection.' : 'Connect a remote MCP server and review its access before AI Code can use it.'"
    :ui="{ content: 'sm:max-w-2xl' }"
  >
    <template #body>
      <UForm
        id="mcp-connection-form"
        :schema="schema"
        :state="state"
        class="space-y-5"
        @submit="onSubmit"
      >
        <template v-if="!isEditing">
          <UFormField
            label="Name"
            name="name"
            required
          >
            <UInput
              v-model="state.name"
              placeholder="Custom Tool"
              class="w-full"
              autocomplete="off"
            />
          </UFormField>

          <UFormField
            label="Description"
            name="description"
            hint="Optional"
          >
            <UInput
              v-model="state.description"
              placeholder="Explain what it does in a few words"
              class="w-full"
              autocomplete="off"
            />
          </UFormField>

          <div class="space-y-2">
            <p class="text-sm font-medium text-highlighted">
              Connection
            </p>
            <div class="inline-flex rounded-full bg-elevated p-1 text-xs font-medium">
              <span class="rounded-full bg-default px-3 py-1.5 text-highlighted shadow-sm">Server URL</span>
              <span
                class="cursor-not-allowed px-3 py-1.5 text-dimmed"
                title="Secure tunnel setup is not available in AI Code yet"
              >
                Tunnel
              </span>
            </div>
            <UFormField name="url">
              <UInput
                v-model="state.url"
                placeholder="https://example.com/mcp"
                class="w-full font-mono"
                inputmode="url"
                autocomplete="url"
                @blur="inspectAuthorization"
              />
            </UFormField>
          </div>

          <UFormField label="Authentication">
            <USelect
              v-model="authChoice"
              :items="authItems"
              value-key="value"
              class="w-full"
            />
            <template #description>
              <span v-if="discoveringAuth">Inspecting server authorization metadata…</span>
              <span v-else-if="authChoice === 'auto' && oauthDiscovery?.available">OAuth discovered automatically.</span>
              <span v-else-if="authChoice === 'auto'">AI Code will use anonymous access unless the configured server resource requires authorization.</span>
              <span v-else-if="authChoice === 'oauth'">OAuth metadata is discovered from the MCP resource. Credentials stay server-side.</span>
              <span v-else>No Authorization header will be requested from this setup flow.</span>
            </template>
          </UFormField>

          <UCollapsible
            v-if="effectiveAuth === 'oauth'"
            v-model:open="advancedOpen"
            class="rounded-lg border border-default"
          >
            <UButton
              label="Advanced OAuth settings"
              description="Review discovered OAuth endpoints, registration support, and scopes."
              trailing-icon="i-lucide-chevron-down"
              color="neutral"
              variant="ghost"
              block
              class="justify-between p-4 text-left"
            />
            <template #content>
              <SettingsMcpOAuthDetails :discovery="oauthDiscovery" />
            </template>
          </UCollapsible>

          <SettingsMcpRiskAcknowledgement v-model="acknowledgedRisk" />
        </template>

        <template v-else>
          <div class="grid gap-4 sm:grid-cols-2">
            <UFormField
              label="Name"
              name="name"
              required
            >
              <UInput
                v-model="state.name"
                class="w-full"
                autocomplete="off"
              />
            </UFormField>
            <UFormField
              label="Transport"
              name="transport"
              required
            >
              <USelect
                v-model="state.transport"
                :items="[
                  { label: 'Streamable HTTP', value: 'http' },
                  { label: 'SSE (legacy)', value: 'sse' }
                ]"
                value-key="value"
                class="w-full"
              />
            </UFormField>
          </div>

          <UFormField
            label="Description"
            name="description"
            hint="Optional"
          >
            <UInput
              v-model="state.description"
              class="w-full"
              autocomplete="off"
            />
          </UFormField>

          <UFormField
            label="Server URL"
            name="url"
            required
            description="Changing the endpoint requires a fresh tool scan before it can be saved."
          >
            <UInput
              v-model="state.url"
              class="w-full font-mono"
              inputmode="url"
              autocomplete="url"
            />
          </UFormField>

          <div
            v-if="scanResult"
            class="rounded-lg border border-default bg-elevated/40 p-4"
          >
            <div class="flex items-center justify-between gap-2">
              <div>
                <p class="text-sm font-medium text-highlighted">
                  Tools discovered
                </p>
                <p class="text-xs text-muted">
                  {{ scanResult.tools.length }} {{ scanResult.tools.length === 1 ? 'tool' : 'tools' }} ready.
                </p>
              </div>
              <UBadge
                label="Verified"
                color="success"
                variant="subtle"
              />
            </div>
          </div>

          <p
            v-if="connectionChanged && !scanFresh"
            class="text-xs text-muted"
          >
            Scan the updated endpoint before saving connection changes.
          </p>
        </template>

        <UAlert
          v-if="errorMessage"
          title="Could not connect MCP server"
          :description="errorMessage"
          icon="i-lucide-circle-alert"
          color="error"
          variant="subtle"
        />
      </UForm>
    </template>

    <template #footer>
      <div class="flex w-full items-center justify-between gap-3">
        <UButton
          label="Cancel"
          color="neutral"
          variant="ghost"
          @click="open = false"
        />

        <div v-if="!isEditing">
          <UButton
            label="Create"
            icon="i-lucide-plus"
            type="submit"
            form="mcp-connection-form"
            :loading="scanning || saving"
            :disabled="!canCreate"
          />
        </div>

        <div
          v-else
          class="flex gap-2"
        >
          <UButton
            :label="scanFresh ? 'Scan again' : 'Scan tools'"
            icon="i-lucide-scan-search"
            color="neutral"
            variant="outline"
            type="submit"
            form="mcp-connection-form"
            :loading="scanning"
            :disabled="saving"
            @click="submitIntent = 'scan'"
          />
          <UButton
            label="Save changes"
            icon="i-lucide-save"
            type="submit"
            form="mcp-connection-form"
            :loading="saving"
            :disabled="!canSave || scanning"
            @click="submitIntent = 'save'"
          />
        </div>
      </div>
    </template>
  </UModal>
</template>
