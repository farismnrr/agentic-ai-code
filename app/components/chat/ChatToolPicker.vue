<script setup lang="ts">
const modelValue = defineModel<string[]>({ default: () => [] })

const { servers } = useMcpServers()

/** Only connected, enabled MCP servers can offer tools. */
const usable = computed(() =>
  servers.value.filter(server => server.enabled && server.status === 'connected')
)

// Persisted tool IDs can outlive the catalog entries they point at. Count only
// capabilities the user can actually see and toggle in this picker.
const count = computed(() => {
  const validIds = new Set(usable.value.flatMap(server => server.tools.map(tool => tool.id)))
  return modelValue.value.filter(id => validIds.has(id)).length
})

function isOn(toolId: string) {
  return modelValue.value.includes(toolId)
}

function setTool(toolId: string, enabled: boolean) {
  modelValue.value = enabled
    ? [...new Set([...modelValue.value, toolId])]
    : modelValue.value.filter(id => id !== toolId)
}

function serverState(serverId: string): boolean | 'indeterminate' {
  const tools = usable.value.find(server => server.id === serverId)?.tools ?? []
  const on = tools.filter(tool => isOn(tool.id)).length
  if (on === 0) return false
  return on === tools.length ? true : 'indeterminate'
}

function toggleServer(serverId: string) {
  const tools = usable.value.find(server => server.id === serverId)?.tools ?? []
  const ids = tools.map(tool => tool.id)
  const allOn = serverState(serverId) === true
  modelValue.value = allOn
    ? modelValue.value.filter(id => !ids.includes(id))
    : [...new Set([...modelValue.value, ...ids])]
}
</script>

<template>
  <UPopover :content="{ align: 'start' }">
    <UButton
      icon="i-lucide-blocks"
      :label="count ? `${count} ${count === 1 ? 'tool' : 'tools'}` : 'Tools'"
      color="neutral"
      variant="ghost"
      size="sm"
    />

    <template #content>
      <div class="max-h-96 w-80 overflow-y-auto p-2">
        <div>
          <div class="px-2 py-1 text-xs font-medium uppercase tracking-wide text-dimmed">
            Remote MCP tools
          </div>

          <p
            v-if="!usable.length"
            class="px-2 py-2 text-sm text-muted"
          >
            No connected MCP servers. Add one in
            <ULink to="/settings/mcp">
              settings
            </ULink>.
          </p>

          <div
            v-for="server in usable"
            :key="server.id"
            class="mb-2"
          >
            <UCheckbox
              :model-value="serverState(server.id)"
              :label="server.name"
              :ui="{ label: 'font-medium' }"
              class="px-2 py-1"
              @update:model-value="toggleServer(server.id)"
            />

            <UCheckbox
              v-for="tool in server.tools"
              :key="tool.id"
              :model-value="isOn(tool.id)"
              :label="tool.name"
              :description="tool.description"
              class="px-2 py-1 ps-6"
              @update:model-value="setTool(tool.id, Boolean($event))"
            />
          </div>
        </div>
      </div>
    </template>
  </UPopover>
</template>
