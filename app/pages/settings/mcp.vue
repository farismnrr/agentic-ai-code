<script setup lang="ts">
import * as v from 'valibot'
import type { FormSubmitEvent } from '@nuxt/ui'
import type { McpTransport } from '#shared/types/chat'

useSeoMeta({ title: 'MCP servers' })

const { servers, setEnabled, add, test, remove } = useMcpServers()
const toast = useToast()

const statusColor = {
  connected: 'success',
  connecting: 'warning',
  disconnected: 'neutral',
  error: 'error'
} as const

const addOpen = ref(false)
const testingId = ref<string | null>(null)

const schema = v.pipe(
  v.object({
    name: v.pipe(v.string(), v.minLength(1, 'Name is required')),
    description: v.string(),
    transport: v.picklist(['http', 'sse', 'stdio'] as const),
    url: v.optional(v.string()),
    command: v.optional(v.string())
  }),
  // stdio needs a command, http/sse need a URL — the field that matters
  // depends on the transport, so it can't be a per-field rule.
  v.forward(
    v.check(
      input => input.transport !== 'stdio' || Boolean(input.command?.trim()),
      'A command is required for stdio servers'
    ),
    ['command']
  ),
  v.forward(
    v.check(
      input => input.transport === 'stdio' || Boolean(input.url?.trim()),
      'A URL is required for http and sse servers'
    ),
    ['url']
  )
)

type Schema = v.InferOutput<typeof schema>

const state = reactive<{
  name: string
  description: string
  transport: McpTransport
  url: string
  command: string
}>({ name: '', description: '', transport: 'http', url: '', command: '' })

async function onSubmit(event: FormSubmitEvent<Schema>) {
  await add({
    name: event.data.name,
    description: event.data.description,
    transport: event.data.transport,
    url: event.data.url || undefined,
    command: event.data.command || undefined,
    enabled: true
  })
  toast.add({ title: `Added ${event.data.name}`, description: 'Test the connection to discover its tools.', icon: 'i-lucide-check', color: 'success' })
  addOpen.value = false
  Object.assign(state, { name: '', description: '', transport: 'http', url: '', command: '' })
}

async function testServer(id: string, name: string) {
  testingId.value = id
  try {
    const result = await test(id)
    toast.add({
      title: `${name} connected`,
      description: `${result.tools.length} tools discovered.`,
      icon: 'i-lucide-plug-zap',
      color: 'success'
    })
  } catch {
    toast.add({
      title: `Could not connect to ${name}`,
      description: 'Check the endpoint, tunnel, and server-side OAuth configuration.',
      icon: 'i-lucide-circle-alert',
      color: 'error'
    })
  } finally {
    testingId.value = null
  }
}

async function removeServer(id: string, name: string) {
  await remove(id)
  toast.add({ title: `Removed ${name}`, icon: 'i-lucide-trash-2', color: 'neutral' })
}
</script>

<template>
  <div class="space-y-4 py-4">
    <div class="flex flex-col sm:flex-row sm:items-start justify-between gap-4">
      <div>
        <h2 class="text-base font-semibold text-highlighted">
          MCP servers
        </h2>
        <p class="text-sm text-muted">
          Tools from connected servers are offered to the model. Disable a
          server to withdraw its tools from every conversation.
        </p>
      </div>

      <UButton
        label="Add server"
        icon="i-lucide-plus"
        @click="addOpen = true"
      />
    </div>

    <UCard
      v-if="!servers.length"
      class="border-dashed"
      :ui="{ body: 'flex flex-col items-center justify-center py-12 text-center' }"
    >
      <div class="mb-4 flex size-10 items-center justify-center rounded-full bg-elevated">
        <UIcon
          name="i-lucide-blocks"
          class="size-5 text-muted"
        />
      </div>
      <h3 class="mb-1 text-sm font-medium text-highlighted">
        No MCP servers
      </h3>
      <p class="mb-4 text-sm text-muted">
        Add a server to give the model tools to work with.
      </p>
      <UButton
        label="Add server"
        icon="i-lucide-plus"
        color="neutral"
        variant="outline"
        @click="addOpen = true"
      />
    </UCard>

    <UCard
      v-for="server in servers"
      :key="server.id"
      :ui="{ body: 'space-y-3' }"
    >
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <p class="font-medium text-highlighted">
              {{ server.name }}
            </p>
            <UBadge
              :label="server.status"
              :color="statusColor[server.status]"
              variant="subtle"
              size="sm"
            />
            <UBadge
              :label="server.transport"
              color="neutral"
              variant="outline"
              size="sm"
            />
          </div>
          <p class="text-sm text-muted">
            {{ server.description }}
          </p>
          <code class="text-xs break-all text-dimmed">{{ server.url ?? server.command }}</code>
        </div>

        <div class="flex items-center gap-2">
          <UButton
            icon="i-lucide-plug-zap"
            color="neutral"
            variant="ghost"
            size="sm"
            :loading="testingId === server.id"
            :disabled="server.transport === 'stdio' || (testingId !== null && testingId !== server.id)"
            :aria-label="`Test ${server.name}`"
            @click="testServer(server.id, server.name)"
          />
          <USwitch
            :model-value="server.enabled"
            :aria-label="`Enable ${server.name}`"
            @update:model-value="setEnabled(server.id, $event)"
          />
          <UButton
            icon="i-lucide-trash-2"
            color="neutral"
            variant="ghost"
            size="sm"
            :aria-label="`Remove ${server.name}`"
            @click="removeServer(server.id, server.name)"
          />
        </div>
      </div>

      <UCollapsible v-if="server.tools.length">
        <UButton
          :label="`${server.tools.length} tools`"
          icon="i-lucide-chevron-down"
          color="neutral"
          variant="ghost"
          size="xs"
        />

        <template #content>
          <div class="space-y-1 pt-2">
            <div
              v-for="tool in server.tools"
              :key="tool.id"
              class="rounded-md px-2 py-1 hover:bg-elevated"
            >
              <code class="text-sm text-highlighted">{{ tool.name }}</code>
              <p class="text-xs text-muted">
                {{ tool.description }}
              </p>
            </div>
          </div>
        </template>
      </UCollapsible>

      <p
        v-else
        class="text-xs text-dimmed"
      >
        No tools discovered. Test the connection after adding or changing the server.
      </p>
    </UCard>

    <UModal
      v-model:open="addOpen"
      title="Add MCP server"
    >
      <template #body>
        <UForm
          id="add-server"
          :schema="schema"
          :state="state"
          class="space-y-4"
          @submit="onSubmit"
        >
          <UFormField
            label="Name"
            name="name"
            required
          >
            <UInput
              v-model="state.name"
              placeholder="Laptop relay"
              class="w-full"
            />
          </UFormField>

          <UFormField
            label="Description"
            name="description"
          >
            <UInput
              v-model="state.description"
              placeholder="Remote coding tools on my laptop"
              class="w-full"
            />
          </UFormField>

          <UFormField
            label="Transport"
            name="transport"
          >
            <USelect
              v-model="state.transport"
              :items="['http', 'sse', 'stdio']"
              class="w-full"
            />
          </UFormField>

          <UFormField
            v-if="state.transport === 'stdio'"
            label="Command"
            name="command"
          >
            <UInput
              v-model="state.command"
              placeholder="npx -y @modelcontextprotocol/server-filesystem ~/"
              class="w-full"
            />
          </UFormField>

          <UFormField
            v-else
            label="URL"
            name="url"
            :description="state.transport === 'http' ? 'For the first-party laptop relay, use the exact NUXT_REMOTE_MCP_URL configured on the Nuxt server. Its OAuth access token remains server-only.' : undefined"
          >
            <UInput
              v-model="state.url"
              placeholder="https://mcp.farismunir.my.id/mcp"
              class="w-full"
            />
          </UFormField>
        </UForm>
      </template>

      <template #footer>
        <div class="flex w-full justify-end gap-2">
          <UButton
            label="Cancel"
            color="neutral"
            variant="ghost"
            @click="addOpen = false"
          />
          <UButton
            label="Add server"
            type="submit"
            form="add-server"
          />
        </div>
      </template>
    </UModal>
  </div>
</template>
