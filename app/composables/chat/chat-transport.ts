import { DefaultChatTransport } from 'ai'
import type { UIMessage } from '#shared/types/chat'

export function createConversationTransport() {
  return new DefaultChatTransport({
    api: '/api/chat',
    prepareSendMessagesRequest: ({ id, messages, trigger, messageId }) => ({
      body: { id, trigger, messageId, message: messages[messages.length - 1] as UIMessage | undefined }
    })
  })
}
