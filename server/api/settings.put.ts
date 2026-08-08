import * as v from 'valibot'
import { updateSettings } from '../utils/settings'

const settingsSchema = v.object({
  language: v.optional(v.string()),
  streaming: v.optional(v.boolean()),
  sendOnEnter: v.optional(v.boolean()),
  defaultModelId: v.optional(v.string()),
  temperature: v.optional(v.number()),
  systemPrompt: v.optional(v.string()),
  displayName: v.optional(v.string()),
  email: v.optional(v.string())
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readValidatedBody(event, body => v.parse(settingsSchema, body))
  return updateSettings(session.user.id, body)
})
