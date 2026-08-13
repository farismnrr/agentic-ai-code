import { notFound, internal } from '#server/core/errors/http'
import { eq } from 'drizzle-orm'
import { userSettings, users } from '../../database/schema'

export async function getSettings(userId: string, name: string = 'User', email: string = '') {
  const db = useDb()

  const [settings] = await db
    .select()
    .from(userSettings)
    .where(eq(userSettings.userId, userId))
    .limit(1)

  const [userRow] = await db
    .select({ lastActiveWorkspaceId: users.lastActiveWorkspaceId })
    .from(users)
    .where(eq(users.id, userId))
    .limit(1)

  const lastActiveWorkspaceId = userRow?.lastActiveWorkspaceId || null

  if (settings) {
    return {
      language: settings.language,
      streaming: settings.streaming,
      sendOnEnter: settings.sendOnEnter,
      defaultModelId: settings.defaultModelId,
      temperature: settings.temperature,
      systemPrompt: settings.systemPrompt,
      displayName: settings.displayName,
      email: settings.email,
      lastActiveWorkspaceId
    }
  }

  // Create default settings if not exist
  const defaultSettings = {
    userId,
    language: 'en',
    streaming: true,
    sendOnEnter: true,
    defaultModelId: null,
    temperature: 0.7,
    systemPrompt: '',
    displayName: name,
    email: email
  }

  const [newSettings] = await db.insert(userSettings).values(defaultSettings).returning()
  if (!newSettings) {
    throw internal('Failed to create settings')
  }

  return {
    language: newSettings.language,
    streaming: newSettings.streaming,
    sendOnEnter: newSettings.sendOnEnter,
    defaultModelId: newSettings.defaultModelId,
    temperature: newSettings.temperature,
    systemPrompt: newSettings.systemPrompt,
    displayName: newSettings.displayName,
    email: newSettings.email,
    lastActiveWorkspaceId
  }
}

export async function updateSettings(userId: string, updates: { language?: string, streaming?: boolean, sendOnEnter?: boolean, defaultModelId?: string | null, temperature?: number, systemPrompt?: string, displayName?: string, email?: string }) {
  const db = useDb()
  const [updatedSettings] = await db
    .update(userSettings)
    .set(updates)
    .where(eq(userSettings.userId, userId))
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
}
