import { useChat } from '@ai-sdk/vue'
import { lastAssistantMessageIsCompleteWithApprovalResponses } from 'ai'
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
export function useConversationChat(conversation: Ref<Conversation | undefined>) {
  const { setMessages } = useConversations()

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
    api: '/api/chat',
    body: {
      id: conversationId.value
    },
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
    }
  }))

  // Mirror the SDK's messages back into the store so the sidebar, titles and
  // a later revisit of this conversation all see the same history. The SDK
  // owns the messages during a turn; the store is the durable copy.
  watch(chat.messages, (messages) => {
    if (conversation.value) setMessages(conversation.value.id, messages as UIMessage[])
  }, { deep: true })

  return chat
}
