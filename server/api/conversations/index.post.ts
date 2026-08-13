import { createConversation, resolveOwnedModelContext, resolveOwnedWorkspace } from '../../infrastructure/composition'
import * as v from 'valibot'

const createSchema = v.object({
  title: v.string(),
  modelId: v.string(),
  workspaceId: v.string(),
  mode: v.picklist(['chat', 'agent']),
  reasoningEffort: v.optional(v.picklist(['low', 'medium', 'high', 'max']))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)

  const result = v.safeParse(createSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  // Same-tenant enforcement (Plan 031A findings A/B): a conversation must
  // not be creatable against another user's model/provider or workspace
  // merely because the caller can guess/supply a valid UUID. UI-side model
  // and workspace pickers already only show the user's own rows, but that
  // is not authorization — this is the one authoritative server-side check.
  await resolveOwnedModelContext(session.user.id, body.modelId)
  await resolveOwnedWorkspace(session.user.id, body.workspaceId)

  const [conversation] = await createConversation({
    userId: session.user.id,
    workspaceId: body.workspaceId,
    title: body.title,
    modelId: body.modelId,
    mode: body.mode,
    reasoningEffort: body.reasoningEffort
  })

  if (!conversation) {
    throw internal('Failed to create conversation')
  }

  return {
    id: conversation.id,
    title: conversation.title,
    modelId: conversation.modelId,
    mode: conversation.mode,
    workspaceId: conversation.workspaceId,
    enabledToolIds: conversation.enabledToolIds,
    approvals: conversation.approvals,
    createdAt: conversation.createdAt.getTime(),
    updatedAt: conversation.updatedAt.getTime(),
    messages: []
  }
})
