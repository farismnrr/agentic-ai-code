import { useChat } from '@ai-sdk/vue'
import { lastAssistantMessageIsCompleteWithApprovalResponses, DefaultChatTransport } from 'ai'
import type { Conversation, UIMessage } from '#shared/types/chat'

/**
 * Wires one conversation to the AI SDK.
 *
 * The transport is the only mocked piece — `useChat` and its status machine
 * are the real thing, which is what lets `UChatPromptSubmit` derive send /
 * stop / retry on its own. Swapping in a backend is one line here:
 *
 *   new DefaultChatTransport({ api: '/api/chat' })
 */
const GENERIC_PROVIDER_ERROR = 'The model provider returned an error. Try again, or switch models.'

// AI SDK errors from a failed provider call carry a JSON blob as the
// message (e.g. `[503]: {"error":{"message":"Upstream request failed..."}}`)
// rather than something fit for a toast — pull the human-readable part out
// if present. Some providers (seen via a fallback/router chain) throw a
// non-Error value that both this SDK and the server's own logger (see
// server/utils/logger.ts's errorAttributes) can only stringify as the
// literal text "[object Object]" — that's not a message, so treat it (and
// any other non-informative raw message) the same as no message at all.
function friendlyChatErrorMessage(error: Error): string {
  const match = error.message.match(/\{.*\}/s)
  if (match) {
    try {
      const parsed = JSON.parse(match[0])
      const nested = parsed?.error?.message
      if (typeof nested === 'string' && nested.length > 0) return nested
    } catch {
      // Not JSON after all — fall through to the raw message below.
    }
  }
  if (!error.message || error.message === '[object Object]') return GENERIC_PROVIDER_ERROR
  return error.message
}

export function useConversationChat(conversation: Ref<Conversation | undefined>) {
  const { setMessages, loadOne } = useConversations()
  const toast = useToast()

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
  watch(conversationId, () => {
    seedMessages.value = conversation.value?.messages ?? []
  })

  const chat = useChat(() => ({
    transport: new DefaultChatTransport({
      api: '/api/chat',
      prepareSendMessagesRequest: ({ id, messages, trigger, messageId }) => ({
        body: { id, trigger, messageId, message: messages[messages.length - 1] }
      })
    }),
    id: conversationId.value,
    messages: seedMessages.value as UIMessage[],
    // Without this, `addToolApprovalResponse` only marks the pending part as
    // answered in local state — it never actually sends the follow-up
    // request that resumes the conversation and runs the approved tool, so
    // clicking "Always allow" (or an auto-remembered decision) appeared to
    // do nothing and the turn hung forever. This is the SDK's own official
    // helper for exactly this trigger.
    sendAutomaticallyWhen: lastAssistantMessageIsCompleteWithApprovalResponses,
    onError: (error: Error) => {
      console.error('[chat]', error)
      toast.add({
        title: 'Message failed to send',
        description: friendlyChatErrorMessage(error),
        icon: 'i-lucide-alert-triangle',
        color: 'error'
      })
    }
  }))

  // Mirror the SDK's messages back into the store so the sidebar, titles and
  // a later revisit of this conversation all see the same history. The SDK
  // owns the messages during a turn; the store is the durable copy.
  let debounceTimer: ReturnType<typeof setTimeout>

  const flushMessages = () => {
    clearTimeout(debounceTimer)
    if (conversation.value) {
      setMessages(conversation.value.id, chat.messages.value as UIMessage[])
    }
  }

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
    clearTimeout(debounceTimer)
    debounceTimer = setTimeout(flushMessages, 300)
  })

  watch(() => chat.status.value, (status) => {
    if (status !== 'streaming') {
      flushMessages()
      // The mirror-back watcher above only ever patches `messages` — server
      // side fields a turn can also change (lastMeasuredTokens from
      // compaction's usage tracking, contextSummary, approvals persisted
      // mid-turn) never reach the client otherwise, so e.g. the context-usage
      // indicator stayed frozen at whatever it showed on page load. Refetch
      // once per turn, not per chunk — this fires at most as often as
      // `flushMessages` already does.
      if (conversation.value) loadOne(conversation.value.id)
    }
  })

  return chat
}
