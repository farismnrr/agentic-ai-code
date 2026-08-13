<script setup lang="ts">
import { nativeTools, NATIVE_LOCAL_TERMINAL_TOOL_ID } from '#shared/utils/native-tools'
import type { ApprovalDecision } from '#shared/types/chat'

const props = defineProps<{
  approvals?: Record<string, ApprovalDecision>
}>()

const emit = defineEmits<{
  'update:approvals': [value: Record<string, ApprovalDecision>]
}>()

const modelValue = defineModel<string[]>({ default: () => [] })

const { servers } = useMcpServers()

/** Only connected, enabled servers can offer tools. */
const usable = computed(() =>
  servers.value.filter(server => server.enabled && server.status === 'connected')
)

/** The subset of `nativeTools` this component actually renders a checkbox for. */
const visibleNativeTools = computed(() => nativeTools.filter(t => t.pickerVisible !== false))

// `modelValue` (conv.enabledToolIds) is a persisted array that can outlive
// the tools it names — a tool removed from `nativeTools` (or an MCP server
// disconnected/removed) leaves its id sitting in old conversations forever,
// since nothing retroactively cleans that column up. Counting
// `modelValue.length` directly showed a stale, un-toggleable "2 tools" for
// a conversation that had only one real, visible checkbox. Count only ids
// that still resolve to something this popover actually lists.
const count = computed(() => {
  const validIds = new Set([
    ...visibleNativeTools.value.map(t => t.id),
    ...usable.value.flatMap(s => s.tools.map(t => t.id))
  ])
  return modelValue.value.filter(id => validIds.has(id)).length
})

function isOn(toolId: string) {
  return modelValue.value.includes(toolId)
}

function toggleTool(toolId: string) {
  modelValue.value = isOn(toolId)
    ? modelValue.value.filter(id => id !== toolId)
    : [...modelValue.value, toolId]
}

function serverState(serverId: string): boolean | 'indeterminate' {
  const tools = usable.value.find(s => s.id === serverId)?.tools ?? []
  const on = tools.filter(t => isOn(t.id)).length
  if (on === 0) return false
  return on === tools.length ? true : 'indeterminate'
}

function toggleServer(serverId: string) {
  const tools = usable.value.find(s => s.id === serverId)?.tools ?? []
  const ids = tools.map(t => t.id)
  const allOn = serverState(serverId) === true
  modelValue.value = allOn
    ? modelValue.value.filter(id => !ids.includes(id))
    : [...new Set([...modelValue.value, ...ids])]
}

function resetApproval(toolId: string) {
  if (!props.approvals) return
  const { [toolId]: _, ...next } = props.approvals
  emit('update:approvals', next)
}

// This is NOT a "can the AI use local_terminal at all" switch — that's
// decided entirely server-side by whether the user has a paired device
// (see server/api/chat.post.ts), same as every other native tool here has
// no on/off toggle of its own. This one control is specifically about
// whether every command still needs its own approval-modal click: writes
// the exact same `conversations.approvals` entry
// ChatToolApproval.vue's own "Always allow" button writes, just reachable
// proactively instead of only after a first prompt. Per-conversation only,
// same as that column always is — flipping it here never touches any other
// conversation.
const skipLocalTerminalApproval = computed(() => props.approvals?.[NATIVE_LOCAL_TERMINAL_TOOL_ID] === 'always')

function toggleSkipLocalTerminalApproval(value: boolean) {
  if (value) {
    emit('update:approvals', { ...props.approvals, [NATIVE_LOCAL_TERMINAL_TOOL_ID]: 'always' })
  } else {
    resetApproval(NATIVE_LOCAL_TERMINAL_TOOL_ID)
  }
}
</script>

<template>
  <UPopover :content="{ align: 'start' }">
    <UButton
      icon="i-lucide-blocks"
      :label="count ? `${count} tools` : 'Tools'"
      color="neutral"
      variant="ghost"
      size="sm"
    />

    <template #content>
      <div class="max-h-96 w-72 overflow-y-auto p-2">
        <p
          v-if="!usable.length"
          class="p-2 text-sm text-muted"
        >
          No connected servers. Add one in
          <ULink to="/settings/mcp">
            settings
          </ULink>.
        </p>

        <div
          v-if="visibleNativeTools.length > 0"
          class="mb-2"
        >
          <div class="px-2 py-1 font-medium text-sm">
            Built-in
          </div>
          <div
            v-for="tool in visibleNativeTools"
            :key="tool.id"
            class="flex items-center justify-between gap-1"
          >
            <UCheckbox
              :model-value="isOn(tool.id)"
              :label="tool.name"
              :description="tool.description"
              class="px-2 py-1 ps-6 flex-1 min-w-0"
              @update:model-value="toggleTool(tool.id)"
            />
            <UBadge
              v-if="approvals?.[tool.id]"
              size="xs"
              :color="approvals[tool.id] === 'always' ? 'primary' : 'error'"
              variant="subtle"
              class="cursor-pointer shrink-0 me-2"
              title="Click to reset approval decision"
              @click.stop="resetApproval(tool.id)"
            >
              {{ approvals[tool.id] }}
              <UIcon
                name="i-lucide-x"
                class="size-3 ms-1"
              />
            </UBadge>
          </div>
        </div>

        <div class="mb-2 border-t border-default pt-2">
          <div class="flex items-center justify-between gap-2 px-2 py-1">
            <div class="min-w-0">
              <p class="text-sm">
                Skip approval for local terminal
              </p>
              <p class="text-xs text-muted">
                Only in this conversation. Doesn't affect whether it's available — that follows Settings → Local Terminal pairing.
              </p>
            </div>
            <USwitch
              :model-value="skipLocalTerminalApproval"
              class="shrink-0"
              @update:model-value="toggleSkipLocalTerminalApproval"
            />
          </div>
        </div>

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

          <div
            v-for="tool in server.tools"
            :key="tool.id"
            class="flex items-center justify-between gap-1"
          >
            <UCheckbox
              :model-value="isOn(tool.id)"
              :label="tool.name"
              :description="tool.description"
              class="px-2 py-1 ps-6 flex-1 min-w-0"
              @update:model-value="toggleTool(tool.id)"
            />
            <UBadge
              v-if="approvals?.[tool.id]"
              size="xs"
              :color="approvals[tool.id] === 'always' ? 'primary' : 'error'"
              variant="subtle"
              class="cursor-pointer shrink-0 me-2"
              title="Click to reset approval decision"
              @click.stop="resetApproval(tool.id)"
            >
              {{ approvals[tool.id] }}
              <UIcon
                name="i-lucide-x"
                class="size-3 ms-1"
              />
            </UBadge>
          </div>
        </div>
      </div>
    </template>
  </UPopover>
</template>
