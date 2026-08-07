import { eq } from 'drizzle-orm'
import { userSettings } from '../database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const db = useDb()

  const [settings] = await db
    .select()
    .from(userSettings)
    .where(eq(userSettings.userId, session.user.id))
    .limit(1)

  if (settings) {
    return {
      language: settings.language,
      streaming: settings.streaming,
      sendOnEnter: settings.sendOnEnter,
      defaultModelId: settings.defaultModelId,
      temperature: settings.temperature,
      systemPrompt: settings.systemPrompt,
      displayName: settings.displayName,
      email: settings.email
    }
  }

  // Create default settings if not exist
  const defaultSettings = {
    userId: session.user.id,
    language: 'en',
    streaming: true,
    sendOnEnter: true,
    defaultModelId: 'gpt-4o-mini', // or another default
    temperature: 0.7,
    systemPrompt: '',
    displayName: session.user.name || 'User',
    email: session.user.email || ''
  }

  const [newSettings] = await db.insert(userSettings).values(defaultSettings).returning()

  if (!newSettings) {
    throw createError({ statusCode: 500, message: 'Failed to create settings' })
  }

  return {
    language: newSettings.language,
    streaming: newSettings.streaming,
    sendOnEnter: newSettings.sendOnEnter,
    defaultModelId: newSettings.defaultModelId,
    temperature: newSettings.temperature,
    systemPrompt: newSettings.systemPrompt,
    displayName: newSettings.displayName,
    email: newSettings.email
  }
})
