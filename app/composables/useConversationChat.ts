import { useChat } from '@ai-sdk/vue'
import { lastAssistantMessageIsCompleteWithApprovalResponses, lastAssistantMessageIsCompleteWithToolCalls } from 'ai'
import type { Conversation, UIMessage } from '#shared/types/chat'
import { friendlyChatErrorMessage } from '../utils/chat-errors'
import { createAttemptLedger } from './chat/attempt-ledger'
import { createLocalToolController } from './chat/local-tool-controller'
import { createConversationTransport } from './chat/chat-transport'
import { createMessageMirror } from './chat/message-mirror'

/**
 * Wires one conversation to the AI SDK.
 *
 * The transport is the only mocked piece — `useChat` and its status machine
 * are the real thing, which is what lets `UChatPromptSubmit` derive send /
 * stop / retry on its own. Swapping in a backend is one line here:
 *
 *   new DefaultChatTransport({ api: '/api/chat' })
 */
// Provider/transport error messages are private diagnostics and may contain
// prompts, database details, paths, or user input. The UI receives only the
// stable category returned by friendlyChatErrorMessage().
export function useConversationChat(conversation: Ref<Conversation | undefined>) {
  const { setMessages, loadOne } = useConversations()
  const toast = useToast()
  const relayAgent = useRelayAgent()

  // `useChat`'s options factory re-runs — recreating its whole internal
  // chat instance and resetting `status` back to 'ready' — whenever ANY
  // reactive value read inside it is invalidated, not only when that value
  // actually changes. `conversation` is a single ref reassigned wholesale
  // on every streamed chunk (useConversations.ts's updateLocally replaces
  // the whole array/object), including by the mirror-back watch just below
  // this. Reading `conversation.value.id`/`.messages` directly here was
  // therefore re-triggering this factory on every chunk mid-stream, which
  // silently discarded `status` transitions before Vue ever painted them —
  // no loading indicator could ever show, no matter how it was built.
  // Route both through primitives that only change when the conversation
  // being viewed actually changes, not on its own content updates:
  // `conversationId` is a value-deduped computed (Vue computeds only
  // notify consumers when their result actually differs), and
  // `seedMessages` is a snapshot only re-taken when that id changes.
  const conversationId = computed(() => conversation.value?.id)
  const seedMessages = shallowRef<UIMessage[]>(conversation.value?.messages ?? [])
  const agentContext = shallowRef<{ repository_identity?: string } | undefined>()
  watch(conversationId, () => {
    seedMessages.value = conversation.value?.messages ?? []
    agentContext.value = undefined
  })
  watch(conversationId, (id) => {
    if (!id || conversation.value?.mode !== 'agent') return
    void relayAgent.startSession(id).then((result) => {
      const context = result?.context
      if (context && typeof context === 'object' && 'repository_identity' in context && typeof context.repository_identity === 'string') {
        agentContext.value = { repository_identity: context.repository_identity.slice(0, 512) }
      }
    }).catch(() => {})
  }, { immediate: true })
  const stopContinuationUsed = ref(false)
  const completionGateInFlight = ref(false)
  watch(conversationId, () => {
    stopContinuationUsed.value = false
  })

  // `local_terminal` is registered server-side with no `execute` (see
  // server/api/chat.post.ts) precisely so this server never runs the user's
  // local command — once approved, the SDK streams the tool call to the
  // client to run instead. It is deliberately NOT run from `onToolCall`:
  // the SDK fires that the instant a tool call's *input* is available,
  // completely independent of approval — traced through
  // node_modules/ai/dist/index.js and confirmed live (a chat turn stuck at
  // "Still working…" forever, no approval modal ever shown) that this SDK
  // behavior does not gate client-tool execution behind approval at all.
  // Executing from `onToolCall` therefore risked running a command before,
  // or regardless of, whatever the user actually decided in
  // ChatToolApproval.vue's modal. Instead, a real command only ever runs
  // from the watcher below, which fires strictly on the message part
  // actually reaching `state: 'approval-responded'` with
  // `approval.approved === true` — true whether that came from the user
  // clicking Allow just now, or from a remembered `conv.approvals` decision
  // the server already auto-resolved (both produce the same state
  // transition, just in different turns). A denied response needs no
  // handling here — the SDK resolves `output-denied` on its own without
  // ever invoking a client tool.
  const ledger = createAttemptLedger()
  const hookApproval = ref<{ input: unknown, token: string }>()
  let resolveHookApproval: ((approved: boolean) => void) | undefined
  const requestHookApproval = (input: unknown, token: string) => new Promise<boolean>((resolve) => {
    resolveHookApproval = resolve
    hookApproval.value = { input, token }
  })
  const answerHookApproval = (approved: boolean) => {
    resolveHookApproval?.(approved)
    resolveHookApproval = undefined
    hookApproval.value = undefined
  }

  // `executedLocalTerminalCalls` alone only guards within one page
  // session — the `{ immediate: true }` watcher below is what makes
  // reopening a conversation resume a call that was approved but never
  // got to run (e.g. the tab closed mid-flight), which is the behavior we
  // want. But that same resume-on-reload logic has a narrow failure
  // window: if a command actually finished running on the CLI but the
  // follow-up request that would persist its output never completed
  // (network dropped at exactly that moment), the DB row is left looking
  // identical to "never ran yet" — reopening would then run it again.
  // A durable, cross-reload ledger closes that window: mark a call as
  // attempted here, synchronously, before ever awaiting `exec()` — so even
  // a mid-flight crash right after this line means a reload will never
  // retry it, at the cost of never auto-retrying a call that setup-failed
  // for some unrelated reason (acceptable; the model still sees the error
  // via `addToolOutput` below in that case, it just won't self-heal on
  // reload — a real duplicate command run is the worse failure mode here).

  const chat = useChat(() => ({
    transport: createConversationTransport(agentContext),
    id: conversationId.value,
    messages: seedMessages.value as UIMessage[],
    // Without this, `addToolApprovalResponse` only marks the pending part as
    // answered in local state — it never actually sends the follow-up
    // request that resumes the conversation and runs the approved tool, so
    // clicking "Always allow" (or an auto-remembered decision) appeared to
    // do nothing and the turn hung forever. This is the SDK's own official
    // helper for exactly this trigger. `local_terminal`'s result arrives via
    // `addToolOutput` above rather than an approval response, so both
    // "turn is ready to resume" conditions are checked — either one being
    // true is enough to send the follow-up request.
    sendAutomaticallyWhen: options =>
      lastAssistantMessageIsCompleteWithApprovalResponses(options) || lastAssistantMessageIsCompleteWithToolCalls(options),
    onError: (error: Error) => {
      toast.add({
        title: 'Message failed to send',
        description: friendlyChatErrorMessage(error),
        icon: 'i-lucide-alert-triangle',
        color: 'error'
      })
    }
  }))

  const runApprovedLocalTerminalCall = createLocalToolController({ chat, relayAgent, ledger, agentSession: conversationId.value, requestApproval: requestHookApproval })

  // The one place `local_terminal` actually gets executed — see the note
  // above `runApprovedLocalTerminalCall`. Fires on every message mutation
  // (cheap: `executedLocalTerminalCalls` skips anything already handled),
  // scanning for a part that just became genuinely approved. `immediate:
  // true` because plain `watch()` only reacts to *changes* — without it, a
  // conversation reopened with an already-approved-but-never-executed call
  // still sitting in its loaded history (e.g. the tab closed before this
  // ever got a chance to run) would never resume it, only a fresh mutation
  // would ever be seen.
  watch(chat.messages, () => {
    for (const message of chat.messages.value) {
      for (const part of message.parts) {
        if (part.type !== 'tool-local_terminal') continue
        const p = part as unknown as { state?: string, approval?: { approved?: boolean }, toolCallId: string, input: unknown }
        if (p.state === 'approval-responded' && p.approval?.approved === true) {
          void runApprovedLocalTerminalCall(p)
        }
      }
    }
  }, { immediate: true })

  // Mirror the SDK's messages back into the store so the sidebar, titles and
  // a later revisit of this conversation all see the same history. The SDK
  // owns the messages during a turn; the store is the durable copy.
  const mirror = createMessageMirror({ conversation, messages: chat.messages, setMessages })
  const flushMessages = mirror.flush

  // Not `{ deep: true }`: `chat.messages` is a `shallowRef` and the SDK
  // itself calls `triggerRef()` on every push/pop/replace
  // (@ai-sdk/vue's `pushMessage`/`popMessage`/`replaceMessage`), which
  // already forces this watcher to fire on every mutation regardless of
  // the deep option. `deep: true` bought nothing here except making Vue
  // `traverse()` — walk every message and every part — on every single
  // streamed chunk before this callback (and its debounce) ever runs,
  // which was the other half of the freeze: cost proportional to total
  // conversation size, paid per token, un-throttleable by debouncing the
  // callback body alone.
  watch(chat.messages, () => {
    mirror.schedule()
  })

  watch(() => chat.status.value, (status, previousStatus) => {
    if (status !== 'streaming') {
      if (previousStatus === 'streaming' && conversationId.value && conversation.value?.mode === 'agent' && !completionGateInFlight.value) {
        completionGateInFlight.value = true
        void relayAgent.preAgentStop(conversationId.value).then((decision) => {
          if (decision.completion === 'remediation_required' && !stopContinuationUsed.value) {
            stopContinuationUsed.value = true
            void chat.regenerate()
            return
          }
          flushMessages()
          if (conversation.value) loadOne(conversation.value.id)
        }).catch(() => {
          flushMessages()
          if (conversation.value) loadOne(conversation.value.id)
        }).finally(() => {
          completionGateInFlight.value = false
        })
      } else if (previousStatus !== 'streaming' || conversation.value?.mode !== 'agent') {
        flushMessages()
        if (conversation.value) loadOne(conversation.value.id)
      }
      // The mirror-back watcher above only ever patches `messages` — server
      // side fields a turn can also change (lastMeasuredTokens from
      // compaction's usage tracking, contextSummary, approvals persisted
      // mid-turn) never reach the client otherwise, so e.g. the context-usage
      // indicator stayed frozen at whatever it showed on page load. Refetch
      // once per turn, not per chunk — this fires at most as often as
      // `flushMessages` already does.
    }
  })

  return { ...chat, hookApproval, answerHookApproval }
}
