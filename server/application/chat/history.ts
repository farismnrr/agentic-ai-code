import { insertUserMessage, loadHistoryMessages } from '../../infrastructure/database/chat'
import type { findUserConversation } from '../../infrastructure/database/chat'
import type { UIMessage } from '#shared/types/chat'

type Conversation = NonNullable<Awaited<ReturnType<typeof findUserConversation>>>

/**
 * Decides how a chat turn's `trigger` shapes the message list sent to the
 * model — submit/regenerate/resume semantics. This is application-owned
 * business logic (Plan 031A finding G): infrastructure only exposes plain
 * history reads/writes (`loadHistoryMessages`, `insertUserMessage`), it
 * does not decide what a `submit-message` vs `regenerate-message` vs resume
 * trigger means for the turn.
 */
export async function buildTurnMessages(conversation: Conversation, trigger: string | undefined, message: UIMessage | undefined) {
  let messages = await loadHistoryMessages(conversation)
  if (trigger === 'submit-message' && message?.role === 'user') {
    const inserted = await insertUserMessage(conversation.id, message)
    messages.push({ ...message, id: inserted.id })
  } else if (trigger === 'regenerate-message') {
    if (messages.at(-1)?.role === 'assistant') messages = messages.slice(0, -1)
  } else if (message && messages.length > 0) {
    messages[messages.length - 1] = message
  }
  return messages
}
