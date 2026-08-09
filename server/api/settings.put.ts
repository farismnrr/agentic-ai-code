import * as v from 'valibot'
import { updateSettings } from '../utils/settings'

const settingsSchema = v.object({
  language: v.optional(v.string()),
  streaming: v.optional(v.boolean()),
  sendOnEnter: v.optional(v.boolean()),
  defaultModelId: v.optional(v.nullable(v.string())),
  temperature: v.optional(v.number()),
  systemPrompt: v.optional(v.string()),
  displayName: v.optional(v.string()),
  email: v.optional(v.string()),
  defaultContextWindow: v.optional(v.number()),
  defaultMaxOutputTokens: v.optional(v.number()),
  defaultThinkingEnabled: v.optional(v.boolean()),
  defaultThinkingMinTokens: v.optional(v.number()),
  defaultThinkingMaxTokens: v.optional(v.number())
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readBody(event)
  const parsed = v.safeParse(settingsSchema, body)
  if (!parsed.success) {
    throw unprocessable(parsed.issues)
  }
  return updateSettings(session.user.id, parsed.output)
})
