import { eq, and, desc } from 'drizzle-orm'
import { conversations } from '../../database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const db = useDb()

  const query = getQuery(event)
  if (!query.workspaceId || typeof query.workspaceId !== 'string') {
    throw badRequest('Missing workspaceId')
  }

  const userConversations = await db
    .select()
    .from(conversations)
    .where(and(eq(conversations.userId, session.user.id), eq(conversations.workspaceId, query.workspaceId)))
    .orderBy(desc(conversations.updatedAt))

  // For the list, we don't fetch all messages, just the metadata.
  // Wait, in `useConversations.ts` they expect `messages: []` to be present if missing.
  return userConversations.map(c => ({
    id: c.id,
    title: c.title,
    modelId: c.modelId,
    reasoningEffort: c.reasoningEffort,
    enabledToolIds: c.enabledToolIds,
    approvals: c.approvals,
    createdAt: c.createdAt.getTime(),
    updatedAt: c.updatedAt.getTime(),
    messages: [] // messages are fetched when opening the conversation
  }))
})
