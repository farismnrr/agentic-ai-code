import { eq, and } from 'drizzle-orm'
import { conversations } from '../../database/schema'
import * as v from 'valibot'

const updateSchema = v.object({
  title: v.optional(v.string()),
  enabledToolIds: v.optional(v.array(v.string())),
  approvals: v.optional(v.record(v.string(), v.union([v.literal('always'), v.literal('never')])))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing conversation ID')

  const result = v.safeParse(updateSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output
  const db = useDb()

  const [updated] = await db
    .update(conversations)
    .set({
      ...body,
      updatedAt: new Date()
    })
    .where(and(eq(conversations.id, id), eq(conversations.userId, session.user.id)))
    .returning()

  if (!updated) {
    throw notFound('Conversation not found')
  }

  return {
    id: updated.id,
    title: updated.title,
    modelId: updated.modelId,
    enabledToolIds: updated.enabledToolIds,
    approvals: updated.approvals,
    createdAt: updated.createdAt.getTime(),
    updatedAt: updated.updatedAt.getTime()
  }
})
