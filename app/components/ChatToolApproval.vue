<script setup lang="ts">
import { getToolOrDynamicToolName, isToolUIPart } from 'ai'
import { nativeTools } from '#shared/utils/native-tools'
import type { Conversation, UIMessage } from '#shared/types/chat'

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
  answer: [{ id: string, approved: boolean, toolId?: string, remember?: 'always' | 'never' }]
}>()

const { toolsById } = useMcpServers()

interface PendingApproval {
  approvalId: string
  toolName: string
  serverId?: string
  input: unknown
}

// MCP tools resolve an id via `serverId.toolName`; native tools (e.g.
// `terminal`, registered directly in the ToolSet with no MCP server) have no
// `serverId` at all, so that id would always be undefined for them — meaning
// "Always allow"/"Always deny" silently never persisted for the terminal
// tool. Fall back to the native tools registry, matched by its model-facing
// `toolName`, so the id lines up with what server/api/chat.post.ts reads
// back from `conversation.approvals`.
function resolveToolId(toolName: string, serverId: string | undefined) {
  if (serverId) return `${serverId}.${toolName}`
  return nativeTools.find(t => t.toolName === toolName)?.id
}

interface Candidate extends PendingApproval {
  toolId: string | undefined
  /**
   * The AI SDK always streams a `tool-approval-request` chunk (this
   * state) immediately followed by `tool-approval-response` for any call
   * the server already resolved on its own (see
   * node_modules/ai/dist/index.js:7206-7213 — `isAutomatic: true` is set
   * precisely for that case). Normally the second chunk lands a moment
   * later and this component's own auto-answer watch catches up — but if
   * that second chunk is ever delayed relative to this render (confirmed
   * happening in practice on a real multi-tool-call turn), the modal
   * would flash open for an already-decided call.
   */
  isAutomatic: boolean
}

// A single pass over `messages`/`parts`, computed once and reused by both
// `pending` (pure — what to render) and `autoAnswerable` (what to answer
// programmatically) below, so they can never disagree about which part
// they're looking at.
const candidate = computed<Candidate | undefined>(() => {
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
        input: part.input,
        toolId: resolveToolId(toolName, tool?.serverId),
        isAutomatic: part.approval?.isAutomatic === true
      }
    }
  }
  return undefined
})

// What the modal renders: never an automatic resolution (server already
// decided, no user input needed, ever) and never one with a remembered
// decision — those get answered programmatically below instead, without
// ever flashing the modal open first.
const pending = computed<PendingApproval | undefined>(() => {
  const c = candidate.value
  if (!c || c.isAutomatic) return undefined
  const remembered = c.toolId ? props.conversation?.approvals[c.toolId] : undefined
  if (remembered === 'always' || remembered === 'never') return undefined
  return c
})

// What still needs a programmatic `answer()` call despite never being
// shown: a genuinely server-requested approval (not automatic — the
// server didn't resolve it) that this client independently knows a
// remembered decision for. Kept separate from `pending` (which must stay
// side-effect-free) so answering never depends on the modal having
// rendered first.
const autoAnswerable = computed<{ toolId: string, decision: 'always' | 'never' } | undefined>(() => {
  const c = candidate.value
  if (!c || c.isAutomatic || !c.toolId) return undefined
  const remembered = props.conversation?.approvals[c.toolId]
  if (remembered !== 'always' && remembered !== 'never') return undefined
  return { toolId: c.toolId, decision: remembered }
})

const toolId = computed(() => candidate.value?.toolId)

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

  emit('answer', {
    id: current.approvalId,
    approved,
    toolId: toolId.value,
    remember: remember ? (approved ? 'always' : 'never') : undefined
  })
}

// Programmatic answer for a remembered decision the server didn't already
// resolve on its own — never routed through `answer()`/`pending`, since
// this call is never shown. Uses `candidate` directly for the approval id
// so it works regardless of whether `pending` happens to hold anything.
watch(autoAnswerable, (value) => {
  if (!value || !candidate.value) return
  emit('answer', {
    id: candidate.value.approvalId,
    approved: value.decision === 'always',
    toolId: value.toolId
    // no `remember` — already remembered, nothing new to persist
  })
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
