<script setup lang="ts">
import * as v from 'valibot'
import type { FormSubmitEvent } from '@nuxt/ui'
import type { McpRemoteConfig, McpRemoteTransport, McpScanResult, McpServer } from '#shared/types/chat'

const props = defineProps<{
  server?: McpServer | null
  initialKind?: 'local' | 'remote'
}>()

const emit = defineEmits<{
  saved: []
}>()

const open = defineModel<boolean>('open', { default: false })
const { scan, create, update } = useMcpServers()
const { isConfigured: localConfigured, isConnected: localConnected, addLocalRelay } = useRelayAgent()
const toast = useToast()

const schema = v.strictObject({
  name: v.pipe(v.string(), v.trim(), v.minLength(1, 'Name is required'), v.maxLength(80, 'Name is too long')),
  description: v.pipe(v.string(), v.trim(), v.maxLength(280, 'Description is too long')),
  transport: v.picklist(['http', 'sse'] as const),
  url: v.pipe(v.string(), v.trim(), v.minLength(1, 'URL is required'), v.maxLength(2048, 'URL is too long'), v.url('Enter a valid URL'))
})

type Schema = v.InferOutput<typeof schema>
type DialogMode = 'choose' | 'local' | 'remote'

const mode = ref<DialogMode>('choose')
const state = reactive<{ name: string, description: string, transport: McpRemoteTransport, url: string }>({
  name: '',
  description: '',
  transport: 'http',
  url: ''
})
const scanResult = ref<McpScanResult | null>(null)
const scannedKey = ref<string | null>(null)
const scanError = ref<string | null>(null)
const saveError = ref<string | null>(null)
const scanning = ref(false)
const saving = ref(false)
const submitIntent = ref<'scan' | 'save'>('scan')

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
const canSave = computed(() => {
  if (!isEditing.value) return scanFresh.value
  if (!hasChanges.value) return false
  return !connectionChanged.value || scanFresh.value
})
const dialogTitle = computed(() => {
  if (mode.value === 'local') return 'Local relay'
  if (props.server) return `Manage ${props.server.name}`
  return 'Add MCP'
})

watch(currentConnectionKey, () => {
  if (scannedKey.value !== currentConnectionKey.value) {
    scanResult.value = null
    scanError.value = null
  }
})

watch(open, (value) => {
  if (!value) return
  resetDialog()
})

function resetDialog() {
  scanResult.value = null
  scannedKey.value = null
  scanError.value = null
  saveError.value = null
  scanning.value = false
  saving.value = false
  submitIntent.value = 'scan'

  if (props.server) {
    mode.value = 'remote'
    Object.assign(state, {
      name: props.server.name,
      description: props.server.description,
      transport: props.server.transport === 'sse' ? 'sse' : 'http',
      url: props.server.url ?? ''
    })
    return
  }

  mode.value = props.initialKind ?? 'choose'
  Object.assign(state, { name: '', description: '', transport: 'http', url: '' })
}

function choose(kind: 'local' | 'remote') {
  if (kind === 'local' && localConfigured.value) return
  mode.value = kind
  scanError.value = null
  saveError.value = null
}

function finishLocal() {
  if (!localConfigured.value) {
    if (!localConnected.value) return
    addLocalRelay()
    toast.add({
      title: 'Local relay added',
      description: 'This connection is stored only in this browser.',
      icon: 'i-lucide-check',
      color: 'success'
    })
    emit('saved')
  }
  open.value = false
}

function remoteConfig(data: Schema): McpRemoteConfig {
  return {
    name: data.name,
    description: data.description,
    transport: data.transport,
    url: data.url
  }
}

async function onSubmit(event: FormSubmitEvent<Schema>) {
  const config = remoteConfig(event.data)
  saveError.value = null

  if (submitIntent.value === 'scan') {
    scanning.value = true
    scanError.value = null
    try {
      const result = await scan(config)
      scanResult.value = result
      scannedKey.value = `${config.transport}|${config.url}`
    } catch (err: unknown) {
      scanResult.value = null
      scannedKey.value = null
      scanError.value = clientErrorMessage(err, 'Could not connect. Check the URL, transport, and server access, then scan again.')
    } finally {
      scanning.value = false
    }
    return
  }

  if (!canSave.value) return
  saving.value = true
  try {
    if (props.server) {
      await update(props.server.id, config)
      toast.add({ title: 'MCP connection updated', icon: 'i-lucide-check', color: 'success' })
    } else {
      await create(config)
      toast.add({ title: 'MCP connection added', description: `${scanResult.value?.tools.length ?? 0} tools discovered.`, icon: 'i-lucide-check', color: 'success' })
    }
    emit('saved')
    open.value = false
  } catch (err: unknown) {
    saveError.value = clientErrorMessage(err, 'Could not save this MCP connection. Recheck the server and try again.')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <UModal
    v-model:open="open"
    :title="dialogTitle"
    :description="mode === 'remote' ? 'Configure and verify the connection before AI Code uses its tools.' : undefined"
    :ui="{ content: 'sm:max-w-2xl' }"
  >
    <template #body>
      <div
        v-if="mode === 'choose'"
        class="space-y-4"
      >
        <p class="text-sm text-muted">
          Choose what you want to connect. Local relay stays on this device; remote MCP servers are verified by AI Code and saved to your account.
        </p>
        <div class="grid gap-3 sm:grid-cols-2">
          <button
            type="button"
            class="group rounded-lg border border-default p-4 text-left outline-none transition hover:bg-elevated focus-visible:ring-2 focus-visible:ring-primary disabled:cursor-not-allowed disabled:opacity-60 disabled:hover:bg-transparent"
            :disabled="localConfigured"
            @click="choose('local')"
          >
            <div class="flex items-start gap-3">
              <div class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-elevated">
                <UIcon
                  name="i-lucide-laptop"
                  class="size-4 text-muted group-hover:text-highlighted"
                />
              </div>
              <div>
                <p class="text-sm font-medium text-highlighted">
                  Local relay
                </p>
                <p class="mt-1 text-xs leading-5 text-muted">
                  {{ localConfigured ? 'Already added on this browser.' : "Connect the Rust relay running on this browser's Linux device." }}
                </p>
              </div>
            </div>
          </button>
          <button
            type="button"
            class="group rounded-lg border border-default p-4 text-left outline-none transition hover:bg-elevated focus-visible:ring-2 focus-visible:ring-primary"
            @click="choose('remote')"
          >
            <div class="flex items-start gap-3">
              <div class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-elevated">
                <UIcon
                  name="i-lucide-cloud"
                  class="size-4 text-muted group-hover:text-highlighted"
                />
              </div>
              <div>
                <p class="text-sm font-medium text-highlighted">
                  Remote MCP server
                </p>
                <p class="mt-1 text-xs leading-5 text-muted">
                  Add an HTTP or SSE endpoint and scan its tools before saving.
                </p>
              </div>
            </div>
          </button>
        </div>
      </div>

      <SettingsLocalRelaySetup v-else-if="mode === 'local'" />

      <UForm
        v-else
        id="mcp-connection-form"
        :schema="schema"
        :state="state"
        class="space-y-5"
        @submit="onSubmit"
      >
        <div class="grid gap-4 sm:grid-cols-2">
          <UFormField
            label="Name"
            name="name"
            required
          >
            <UInput
              v-model="state.name"
              placeholder="Team tools"
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
                { label: 'SSE', value: 'sse' }
              ]"
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
            placeholder="What this connection provides"
            class="w-full"
            autocomplete="off"
          />
        </UFormField>

        <UFormField
          label="Server URL"
          name="url"
          required
          description="AI Code verifies this endpoint server-side before saving it. Private first-party credentials never leave the server."
        >
          <UInput
            v-model="state.url"
            placeholder="https://mcp.example.com/mcp"
            class="w-full font-mono"
            inputmode="url"
            autocomplete="url"
          />
        </UFormField>

        <UAlert
          v-if="scanError"
          title="Scan failed"
          :description="scanError"
          icon="i-lucide-circle-alert"
          color="error"
          variant="subtle"
        />
        <UAlert
          v-if="saveError"
          title="Could not save connection"
          :description="saveError"
          icon="i-lucide-circle-alert"
          color="error"
          variant="subtle"
        />

        <div
          v-if="scanResult"
          class="rounded-lg border border-default bg-elevated/40 p-4"
        >
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div>
              <p class="text-sm font-medium text-highlighted">
                Tools discovered
              </p>
              <p class="text-xs text-muted">
                {{ scanResult.tools.length }} {{ scanResult.tools.length === 1 ? 'tool' : 'tools' }} ready for review.
              </p>
            </div>
            <UBadge
              label="Verified"
              color="success"
              variant="subtle"
            />
          </div>

          <div
            v-if="scanResult.tools.length"
            class="mt-3 space-y-2"
          >
            <div
              v-for="tool in scanResult.tools.slice(0, 6)"
              :key="tool.name"
              class="rounded-md bg-default px-3 py-2"
            >
              <code class="text-xs font-medium text-highlighted">{{ tool.name }}</code>
              <p
                v-if="tool.description"
                class="mt-0.5 line-clamp-2 text-xs text-muted"
              >
                {{ tool.description }}
              </p>
            </div>
            <p
              v-if="scanResult.tools.length > 6"
              class="px-1 text-xs text-dimmed"
            >
              +{{ scanResult.tools.length - 6 }} more tools
            </p>
          </div>
          <p
            v-else
            class="mt-3 text-xs text-muted"
          >
            The server connected successfully but did not advertise tools.
          </p>
        </div>

        <p
          v-if="connectionChanged && isEditing && !scanFresh"
          class="text-xs text-muted"
        >
          Scan the updated endpoint before saving connection changes.
        </p>
      </UForm>
    </template>

    <template #footer>
      <div class="flex w-full flex-col-reverse gap-2 sm:flex-row sm:items-center sm:justify-between">
        <UButton
          v-if="mode !== 'choose' && !server && !props.initialKind"
          label="Back"
          icon="i-lucide-arrow-left"
          color="neutral"
          variant="ghost"
          class="self-start sm:self-auto"
          @click="mode = 'choose'"
        />
        <div v-else />

        <div class="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <UButton
            label="Cancel"
            color="neutral"
            variant="ghost"
            @click="open = false"
          />
          <UButton
            v-if="mode === 'local'"
            :label="localConfigured ? 'Done' : 'Add MCP'"
            :icon="localConfigured ? undefined : 'i-lucide-plus'"
            :disabled="!localConfigured && !localConnected"
            @click="finishLocal"
          />
          <template v-else-if="mode === 'remote'">
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
              :label="isEditing ? 'Save changes' : 'Add MCP'"
              icon="isEditing ? 'i-lucide-save' : 'i-lucide-plus'"
              type="submit"
              form="mcp-connection-form"
              :loading="saving"
              :disabled="!canSave || scanning"
              @click="submitIntent = 'save'"
            />
          </template>
        </div>
      </div>
    </template>
  </UModal>
</template>
