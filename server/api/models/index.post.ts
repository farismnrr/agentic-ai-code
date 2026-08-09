import * as v from 'valibot'

const bodySchema = v.object({
  providerId: v.string(),
  modelId: v.string(),
  label: v.string(),
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
  const body = await readValidatedBody(event, body => v.parse(bodySchema, body))
  const { providerId, ...modelData } = body
  return createModel(session.user.id, providerId, modelData)
})
