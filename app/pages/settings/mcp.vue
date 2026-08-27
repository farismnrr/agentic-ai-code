<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import type { McpServer } from '#shared/types/chat'

useSeoMeta({ title: 'MCP connections' })

const { servers, loadAll, setEnabled, test, remove } = useMcpServers()
const toast = useToast()
const telemetry = useTelemetry()
const route = useRoute()
const router = useRouter()

const dialogOpen = ref(false)
const dialogServer = ref<McpServer | null>(null)
const testingId = ref<string | null>(null)
const removeCandidate = ref<McpServer | null>(null)
const removing = ref(false)

const hasConnections = computed(() => servers.value.length > 0)

onMounted(async () => {
  const oauth = route.query.oauth
  if (oauth !== 'success' && oauth !== 'error') return

  if (oauth === 'success') {
    telemetry.logEvent('info', 'mcp.oauth.callback', 'MCP OAuth callback completed', {
      'operation': 'mcp.oauth.callback',
      'outcome': 'success',
      'mcp.stage': 'frontend_callback',
      'mcp.oauth': true
    })
    await loadAll().catch(() => undefined)
    toast.add({
      title: 'MCP connected',
      description: 'OAuth sign-in completed and the server tools are ready.',
      icon: 'i-lucide-plug-zap',
      color: 'success'
    })
  } else {
    telemetry.logEvent('error', 'mcp.oauth.callback', 'MCP OAuth callback failed', {
      'operation': 'mcp.oauth.callback',
      'outcome': 'error',
      'mcp.stage': 'frontend_callback',
      'mcp.oauth': true
    })
    toast.add({
      title: 'MCP OAuth failed',
      description: 'The authorization callback could not complete. Start the connection again.',
      icon: 'i-lucide-circle-alert',
      color: 'error'
    })
  }

  await router.replace({ path: '/settings/mcp' })
})

function openAdd() {
  dialogServer.value = null
  dialogOpen.value = true
}

function manageRemote(server: McpServer) {
  dialogServer.value = server
  dialogOpen.value = true
}

function remoteStatus(server: McpServer) {
  return server.transport === 'stdio' ? 'unsupported' : server.status
}

async function recheck(server: McpServer) {
  if (server.transport === 'stdio') return
  testingId.value = server.id
  try {
    const result = await test(server.id)
    toast.add({
      title: `${server.name} connected`,
      description: `${result.tools.length} tools discovered.`,
      icon: 'i-lucide-plug-zap',
      color: 'success'
    })
  } catch (err: unknown) {
    toast.add({
      title: 'Connection check failed',
      description: clientErrorMessage(err, 'Check the server URL and access, then try again.'),
      icon: 'i-lucide-circle-alert',
      color: 'error'
    })
  } finally {
    testingId.value = null
  }
}

async function toggleEnabled(server: McpServer, enabled: boolean) {
  if (server.transport === 'stdio') return
  try {
    await setEnabled(server.id, enabled)
  } catch (err: unknown) {
    toast.add({
      title: 'Could not update connection',
      description: clientErrorMessage(err, 'Try again in a moment.'),
      color: 'error'
    })
  }
}

function serverMenuItems(server: McpServer): DropdownMenuItem[][] {
  if (server.transport === 'stdio') {
    return [[
      { label: 'Remove connection', icon: 'i-lucide-trash-2', color: 'error', onSelect: () => { removeCandidate.value = server } }
    ]]
  }

  return [[
    { label: 'Recheck connection', icon: 'i-lucide-refresh-cw', onSelect: () => { void recheck(server) } },
    { label: 'Remove connection', icon: 'i-lucide-trash-2', color: 'error', onSelect: () => { removeCandidate.value = server } }
  ]]
}

async function confirmRemove() {
  const server = removeCandidate.value
  if (!server) return
  removing.value = true
  try {
    await remove(server.id)
    toast.add({ title: `${server.name} removed`, icon: 'i-lucide-trash-2', color: 'neutral' })
    removeCandidate.value = null
  } catch (err: unknown) {
    toast.add({
      title: 'Could not remove connection',
      description: clientErrorMessage(err, 'Try again in a moment.'),
      color: 'error'
    })
  } finally {
    removing.value = false
  }
}
</script>

<template>
  <div class="space-y-5 py-4">
    <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
      <div class="max-w-2xl">
        <h2 class="text-base font-semibold text-highlighted">
          MCP connections
        </h2>
        <p class="mt-1 text-sm leading-6 text-muted">
          Manage remote MCP servers from one place. Connections are verified before their tools are offered to the model.
        </p>
      </div>
      <UButton
        label="Add MCP"
        icon="i-lucide-plus"
        class="self-start"
        @click="openAdd"
      />
    </div>

    <div
      v-if="!hasConnections"
      class="rounded-lg border border-dashed border-default px-5 py-10 text-center"
    >
      <div class="mx-auto flex size-10 items-center justify-center rounded-full bg-elevated">
        <UIcon
          name="i-lucide-blocks"
          class="size-5 text-muted"
        />
      </div>
      <h3 class="mt-3 text-sm font-medium text-highlighted">
        No MCP connections yet
      </h3>
      <p class="mx-auto mt-1 max-w-md text-sm text-muted">
        Connect a remote MCP server to make its tools available in Agent Mode.
      </p>
      <UButton
        label="Add MCP"
        icon="i-lucide-plus"
        color="neutral"
        variant="outline"
        size="sm"
        class="mt-4"
        @click="openAdd"
      />
    </div>

    <template v-else>
      <div
        v-if="servers.length"
        class="space-y-3"
      >
        <div class="flex items-center justify-between gap-3">
          <p class="text-xs font-medium uppercase tracking-wide text-dimmed">
            Remote servers
          </p>
          <span class="text-xs text-dimmed">{{ servers.length }} configured</span>
        </div>

        <SettingsMcpConnectionCard
          v-for="server in servers"
          :key="server.id"
          :name="server.name"
          :description="server.description || (server.transport === 'stdio' ? 'Legacy server configuration' : 'Remote MCP server')"
          :kind="server.transport === 'stdio' ? 'Legacy stdio' : server.transport === 'http' ? 'HTTP' : 'SSE'"
          :status="remoteStatus(server)"
          :endpoint="server.url ?? server.command"
          :tool-count="server.transport === 'stdio' ? 0 : server.tools.length"
          icon="i-lucide-cloud"
        >
          <template #actions>
            <div
              v-if="server.transport !== 'stdio'"
              class="flex items-center gap-2 rounded-md border border-default px-2.5 py-1.5"
            >
              <span class="text-xs text-muted">Enabled</span>
              <USwitch
                :model-value="server.enabled"
                size="sm"
                :aria-label="`${server.enabled ? 'Disable' : 'Enable'} ${server.name}`"
                @update:model-value="toggleEnabled(server, Boolean($event))"
              />
            </div>
            <UButton
              v-if="server.transport !== 'stdio'"
              label="Manage"
              icon="i-lucide-settings-2"
              color="neutral"
              variant="outline"
              size="sm"
              @click="manageRemote(server)"
            />
            <UDropdownMenu :items="serverMenuItems(server)">
              <UButton
                icon="i-lucide-ellipsis"
                color="neutral"
                variant="ghost"
                size="sm"
                :loading="testingId === server.id"
                :aria-label="`More actions for ${server.name}`"
              />
            </UDropdownMenu>
          </template>

          <UAlert
            v-if="server.transport === 'stdio'"
            title="Unsupported legacy transport"
            description="Server-side stdio execution is intentionally disabled. Remove this legacy entry and add a remote HTTP or SSE server instead."
            icon="i-lucide-shield-alert"
            color="warning"
            variant="subtle"
          />

          <UCollapsible v-else-if="server.tools.length">
            <UButton
              :label="`View ${server.tools.length} ${server.tools.length === 1 ? 'tool' : 'tools'}`"
              icon="i-lucide-chevron-down"
              color="neutral"
              variant="ghost"
              size="xs"
            />
            <template #content>
              <div class="mt-2 grid gap-2 sm:grid-cols-2">
                <div
                  v-for="tool in server.tools"
                  :key="tool.id"
                  class="rounded-md bg-elevated px-3 py-2"
                >
                  <code class="text-xs font-medium text-highlighted">{{ tool.name }}</code>
                  <p
                    v-if="tool.description"
                    class="mt-0.5 line-clamp-2 text-xs text-muted"
                  >
                    {{ tool.description }}
                  </p>
                </div>
              </div>
            </template>
          </UCollapsible>

          <p
            v-else
            class="text-xs text-muted"
          >
            {{ server.status === 'error' ? 'Connection needs attention. Recheck it before using its tools.' : 'Connected server has not advertised any tools.' }}
          </p>
        </SettingsMcpConnectionCard>
      </div>
    </template>

    <SettingsMcpConnectionDialog
      v-model:open="dialogOpen"
      :server="dialogServer"
      @saved="dialogServer = null"
    />

    <UModal
      :open="Boolean(removeCandidate)"
      title="Remove MCP connection"
      description="This removes the saved server and withdraws its tools from conversations."
      @update:open="value => { if (!value) removeCandidate = null }"
    >
      <template #body>
        <p class="text-sm text-muted">
          Remove <span class="font-medium text-highlighted">{{ removeCandidate?.name }}</span>? You can add it again later by scanning the server again.
        </p>
      </template>
      <template #footer>
        <div class="flex w-full justify-end gap-2">
          <UButton
            label="Cancel"
            color="neutral"
            variant="ghost"
            @click="removeCandidate = null"
          />
          <UButton
            label="Remove connection"
            icon="i-lucide-trash-2"
            color="error"
            :loading="removing"
            @click="confirmRemove"
          />
        </div>
      </template>
    </UModal>
  </div>
</template>
