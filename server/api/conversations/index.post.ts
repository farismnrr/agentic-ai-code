import { conversations } from '../../database/schema'
import * as v from 'valibot'

const createSchema = v.object({
  title: v.string(),
  modelId: v.string(),
  workspaceId: v.string(),
  reasoningEffort: v.optional(v.picklist(['low', 'medium', 'high', 'max']))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const db = useDb()

  const result = v.safeParse(createSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  const [conversation] = await db
    .insert(conversations)
    .values({
      userId: session.user.id,
      workspaceId: body.workspaceId,
      title: body.title,
      modelId: body.modelId,
      reasoningEffort: body.reasoningEffort
    })
    .returning()

  if (!conversation) {
    throw internal('Failed to create conversation')
  }

  return {
    id: conversation.id,
    title: conversation.title,
    modelId: conversation.modelId,
    workspaceId: conversation.workspaceId,
    enabledToolIds: conversation.enabledToolIds,
    approvals: conversation.approvals,
    createdAt: conversation.createdAt.getTime(),
    updatedAt: conversation.updatedAt.getTime(),
    messages: []
  }
})
