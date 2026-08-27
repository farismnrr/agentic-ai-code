<script setup lang="ts">
import { getToolOrDynamicToolName, isToolUIPart } from 'ai'
import type { Conversation, UIMessage } from '#shared/types/chat'
import { capabilityFactsForToolCall, classifyCapability, rememberedApprovalCanAutoAnswer } from '#shared/utils/capability-policy'
import { resolveMcpToolFromModelName } from '#shared/utils/mcp-tool-identity'
import { safeInputSummary } from '../../utils/tool-presentation'

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
  toolId?: string
  annotations?: { readOnlyHint?: boolean, destructiveHint?: boolean, openWorldHint?: boolean }
  trustedProvenance?: 'first-party-relay' | 'external'
  identity: 'mcp' | 'unknown'
}

interface Candidate extends PendingApproval {
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
      const mcpTool = resolveMcpToolFromModelName(toolName, Object.values(toolsById.value))

      return {
        approvalId: part.approval.id,
        toolName,
        serverId: mcpTool?.serverId,
        input: part.input,
        toolId: mcpTool?.id,
        annotations: mcpTool?.annotations,
        trustedProvenance: mcpTool?.trustedProvenance,
        identity: mcpTool ? 'mcp' : 'unknown',
        isAutomatic: part.approval?.isAutomatic === true
      }
    }
  }
  return undefined
})

function factsFor(candidate: PendingApproval) {
  return capabilityFactsForToolCall({
    toolId: candidate.toolId ?? candidate.toolName,
    toolName: candidate.toolName,
    input: candidate.input,
    annotations: candidate.annotations,
    // Cached MCP provenance is display metadata only. A request that reached
    // this component is authoritative evidence that the server did not
    // auto-approve it, so MCP is rendered conservatively as external/high-risk.
    trustedProvenance: 'external'
  })
}

// What the modal renders: automatic resolutions and remembered decisions that
// the same shared policy can actually honor stay hidden. A narrowed `always`
// decision remains visible so it can never leave the SDK approval suspended.
const pending = computed<PendingApproval | undefined>(() => {
  const c = candidate.value
  if (!c || c.isAutomatic) return undefined
  const mode = props.conversation?.permissionMode ?? 'manual'
  // The top-level permission selector is authoritative. Bypass deliberately
  // ignores old per-tool remembered answers that may still exist on a
  // conversation from Manual mode.
  const remembered = mode === 'bypass' || !c.toolId ? undefined : props.conversation?.approvals[c.toolId]
  if (remembered === 'never' || (remembered === 'always' && c.identity !== 'mcp' && rememberedApprovalCanAutoAnswer(factsFor(c), remembered, mode))) return undefined
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
  const mode = props.conversation?.permissionMode ?? 'manual'
  if (mode === 'bypass') return undefined
  const remembered = props.conversation?.approvals[c.toolId]
  if (remembered !== 'always' && remembered !== 'never') return undefined
  if (remembered === 'always' && (c.identity === 'mcp' || !rememberedApprovalCanAutoAnswer(factsFor(c), remembered, mode))) return undefined
  return { toolId: c.toolId, decision: remembered }
})

const toolId = computed(() => candidate.value?.toolId)

const open = computed({
  get: () => Boolean(pending.value),
  set: () => { /* closing is only ever a side effect of answering */ }
})

const formattedInput = computed(() => safeInputSummary(pending.value?.input))

const assessment = computed(() => {
  const current = pending.value
  if (!current) return undefined
  return classifyCapability(factsFor(current))
})

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

        <div
          v-if="formattedInput.rows.length || formattedInput.hiddenFields"
          class="space-y-2"
        >
          <p class="font-mono text-[11px] uppercase tracking-wider text-dimmed">
            Scope
          </p>
          <dl
            v-if="formattedInput.rows.length"
            class="grid gap-1.5 text-xs sm:grid-cols-2"
          >
            <div
              v-for="row in formattedInput.rows"
              :key="row.label"
            >
              <dt class="text-dimmed">
                {{ row.label }}
              </dt>
              <dd
                class="truncate font-mono"
                :title="row.value"
              >
                {{ row.value }}
              </dd>
            </div>
          </dl>
          <p
            v-if="formattedInput.hiddenFields"
            class="text-xs text-dimmed"
          >
            {{ formattedInput.hiddenFields }} sensitive/noisy field{{ formattedInput.hiddenFields === 1 ? '' : 's' }} hidden.
          </p>
        </div>

        <div
          v-if="assessment"
          class="flex flex-wrap gap-2"
        >
          <UBadge
            :label="`${assessment.risk} risk`"
            :color="assessment.risk === 'high' ? 'error' : assessment.risk === 'medium' ? 'warning' : 'success'"
            variant="subtle"
          />
          <UBadge
            v-for="effect in assessment.effects"
            :key="effect"
            :label="effect.replaceAll('_', ' ')"
            color="neutral"
            variant="subtle"
          />
          <UBadge
            v-if="assessment.networkRequested"
            label="network requested"
            color="warning"
            variant="subtle"
          />
        </div>

        <p
          v-if="assessment"
          class="text-xs text-dimmed"
        >
          {{ assessment.reason }}. Remembered approvals apply only to low-risk, non-opaque calls.
        </p>

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
