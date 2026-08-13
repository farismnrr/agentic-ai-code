import * as v from 'valibot'
import { createModel } from '../../infrastructure/composition'

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
  const body = await readBody(event)
  const parsed = v.safeParse(bodySchema, body)
  if (!parsed.success) {
    throw unprocessable(parsed.issues)
  }
  const { providerId, ...modelData } = parsed.output
  return createModel(session.user.id, providerId, modelData)
})
