import { eq, desc } from 'drizzle-orm'
import { conversations } from '../../database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const db = useDb()

  const userConversations = await db
    .select()
    .from(conversations)
    .where(eq(conversations.userId, session.user.id))
    .orderBy(desc(conversations.updatedAt))

  // For the list, we don't fetch all messages, just the metadata.
  // Wait, in `useConversations.ts` they expect `messages: []` to be present if missing.
  return userConversations.map(c => ({
    id: c.id,
    title: c.title,
    modelId: c.modelId,
    enabledToolIds: c.enabledToolIds,
    approvals: c.approvals,
    createdAt: c.createdAt.getTime(),
    updatedAt: c.updatedAt.getTime(),
    messages: [] // messages are fetched when opening the conversation
  }))
})
