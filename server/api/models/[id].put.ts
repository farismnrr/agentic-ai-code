import * as v from 'valibot'

const bodySchema = v.object({
  modelId: v.optional(v.string()),
  label: v.optional(v.string()),
  description: v.optional(v.string()),
  icon: v.optional(v.string()),
  contextWindow: v.optional(v.number()),
  maxOutputTokens: v.optional(v.number()),
  thinkingEnabled: v.optional(v.boolean()),
  thinkingMinTokens: v.optional(v.number()),
  thinkingMaxTokens: v.optional(v.number())
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing model ID')
  const body = await readBody(event)
  const parsed = v.safeParse(bodySchema, body)
  if (!parsed.success) {
    throw unprocessable(parsed.issues)
  }
  return event.context.application.models.update(session.user.id, id, parsed.output)
})
