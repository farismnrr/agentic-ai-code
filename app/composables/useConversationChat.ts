import { useChat } from '@ai-sdk/vue'
import type { Conversation, UIMessage } from '~/types/chat'
import { MockChatTransport } from '~/utils/mock-transport'

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
  const settings = useSettings()

  const transport = new MockChatTransport({
    // Both read at send time, so changing tools or the streaming preference
    // mid-conversation takes effect on the next message rather than needing
    // the chat rebuilt.
    getEnabledToolIds: () => conversation.value?.enabledToolIds ?? [],
    getStreaming: () => settings.value.streaming
  })

  const chat = useChat(() => ({
    id: conversation.value?.id,
    messages: (conversation.value?.messages ?? []) as UIMessage[],
    transport,
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
