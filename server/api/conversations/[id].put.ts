import { badRequest, notFound, unprocessable } from '#server/core/errors/http'
import * as v from 'valibot'

const updateSchema = v.object({
  title: v.optional(v.string()),
  modelId: v.optional(v.string()),
  mode: v.optional(v.picklist(['chat', 'agent'])),
  permissionMode: v.optional(v.picklist(['plan', 'workspace', 'autonomous', 'manual'])),
  reasoningEffort: v.optional(v.picklist(['low', 'medium', 'high', 'max'])),
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

  // Same-tenant enforcement (Plan 031A finding A): a model change on an
  // existing conversation must resolve to a model (and backing provider)
  // owned by the same user, not merely any existing model ID.
  if (body.modelId !== undefined) {
    // Ownership is enforced by the application use case.
  }

  const updated = await event.context.application.conversations.update(session.user.id, id, body)

  if (!updated) {
    throw notFound('Conversation not found')
  }

  return {
    id: updated.id,
    title: updated.title,
    modelId: updated.modelId,
    mode: updated.mode,
    reasoningEffort: updated.reasoningEffort,
    enabledToolIds: updated.enabledToolIds,
    approvals: updated.approvals,
    permissionMode: updated.permissionMode,
    createdAt: updated.createdAt.getTime(),
    updatedAt: updated.updatedAt.getTime()
  }
})
