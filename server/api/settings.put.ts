import { eq } from 'drizzle-orm'
import { userSettings } from '../database/schema'
import * as v from 'valibot'

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
  const db = useDb()

  const result = v.safeParse(settingsSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  const [updatedSettings] = await db
    .update(userSettings)
    .set(body)
    .where(eq(userSettings.userId, session.user.id))
    .returning()

  if (!updatedSettings) {
    throw notFound('Settings not found')
  }

  return {
    language: updatedSettings.language,
    streaming: updatedSettings.streaming,
    sendOnEnter: updatedSettings.sendOnEnter,
    defaultModelId: updatedSettings.defaultModelId,
    temperature: updatedSettings.temperature,
    systemPrompt: updatedSettings.systemPrompt,
    displayName: updatedSettings.displayName,
    email: updatedSettings.email
  }
})
