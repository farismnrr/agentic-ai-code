<script setup lang="ts">
import { getToolOrDynamicToolName, isToolUIPart } from 'ai'
import type { Conversation, UIMessage } from '~/types/chat'

/**
 * Approval prompt for a pending MCP tool call.
 *
 * This is a *view over SDK state*, not a state machine of its own. The SDK
 * suspends the tool part in `approval-requested` and only resumes when
 * `addToolApprovalResponse` answers that approval id. A parallel store would
 * look like it worked while leaving the part suspended forever.
 *
 * `conversation.approvals` is deliberately narrow: it remembers only the
 * "always" answers so the dialog can be skipped next time, and is never the
 * record of whether a given call was approved.
 */
const props = defineProps<{
  messages: UIMessage[]
  conversation: Conversation | undefined
}>()

const emit = defineEmits<{
  respond: [{ id: string, approved: boolean }]
  remember: [{ toolId: string, decision: 'always' | 'never' }]
}>()

const { toolsById } = useMcpServers()

interface PendingApproval {
  approvalId: string
  toolName: string
  serverId?: string
  input: unknown
}

const pending = computed<PendingApproval | undefined>(() => {
  for (const message of props.messages) {
    for (const part of message.parts) {
      if (!isToolUIPart(part)) continue
      if (part.state !== 'approval-requested') continue

      const toolName = getToolOrDynamicToolName(part)
      const tool = Object.values(toolsById.value).find(t => t.name === toolName)

      return {
        approvalId: part.approval.id,
        toolName,
        serverId: tool?.serverId,
        input: part.input
      }
    }
  }
  return undefined
})

const toolId = computed(() =>
  pending.value?.serverId ? `${pending.value.serverId}.${pending.value.toolName}` : undefined
)

const open = computed({
  get: () => Boolean(pending.value),
  set: () => { /* closing is only ever a side effect of answering */ }
})

const formattedInput = computed(() =>
  pending.value?.input ? JSON.stringify(pending.value.input, null, 2) : undefined
)

function answer(approved: boolean, remember: boolean) {
  const current = pending.value
  if (!current) return

  if (remember && toolId.value) {
    emit('remember', { toolId: toolId.value, decision: approved ? 'always' : 'never' })
  }
  emit('respond', { id: current.approvalId, approved })
}

// A decision already remembered for this tool answers without interrupting.
watch(pending, (value) => {
  if (!value || !toolId.value) return
  const remembered = props.conversation?.approvals[toolId.value]
  if (remembered === 'always') answer(true, false)
  else if (remembered === 'never') answer(false, false)
}, { immediate: true })
</script>

<template>
  <UModal
    v-model:open="open"
    title="Allow this tool to run?"
    :dismissible="false"
    :close="false"
  >
    <template #body>
      <div
        v-if="pending"
        class="space-y-4"
      >
        <div class="flex items-center gap-2">
          <UBadge
            v-if="pending.serverId"
            :label="pending.serverId"
            color="neutral"
            variant="subtle"
          />
          <code class="text-sm font-medium text-highlighted">{{ pending.toolName }}</code>
        </div>

        <div v-if="formattedInput">
          <p class="mb-1 font-mono text-[11px] uppercase tracking-wider text-dimmed">
            Arguments
          </p>
          <pre class="max-h-56 overflow-auto rounded-md bg-elevated p-2 font-mono text-xs">{{ formattedInput }}</pre>
        </div>

        <p class="text-sm text-muted">
          Tools can read and act on data outside this conversation. Only allow
          calls you understand.
        </p>
      </div>
    </template>

    <template #footer>
      <div class="flex w-full flex-wrap justify-end gap-2">
        <UButton
          label="Deny"
          color="neutral"
          variant="ghost"
          @click="answer(false, false)"
        />
        <UButton
          label="Always deny"
          color="error"
          variant="ghost"
          @click="answer(false, true)"
        />
        <UButton
          label="Allow once"
          color="neutral"
          variant="subtle"
          @click="answer(true, false)"
        />
        <UButton
          label="Always allow"
          color="primary"
          @click="answer(true, true)"
        />
      </div>
    </template>
  </UModal>
</template>
