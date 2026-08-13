import { and, desc, eq } from 'drizzle-orm'
import { apiKeys, conversations, users, workspaces, userDevices } from '../../database/schema'

export async function listConversationSummaries(userId: string, workspaceId?: string) {
  const conditions = [eq(conversations.userId, userId)]
  if (workspaceId) conditions.push(eq(conversations.workspaceId, workspaceId))
  return useDb().select().from(conversations).where(and(...conditions)).orderBy(desc(conversations.updatedAt))
}

export async function listSidebarData(userId: string) {
  const db = useDb()
  return Promise.all([
    db.select().from(workspaces).where(eq(workspaces.userId, userId)).orderBy(desc(workspaces.updatedAt)),
    db.select().from(conversations).where(eq(conversations.userId, userId)).orderBy(desc(conversations.updatedAt))
  ])
}

export async function setActiveWorkspace(userId: string, workspaceId: string | null) {
  await useDb().update(users).set({ lastActiveWorkspaceId: workspaceId }).where(eq(users.id, userId))
}

export async function listUserDevices(userId: string) {
  return useDb().select().from(userDevices).where(eq(userDevices.userId, userId))
}

export async function registerUserDevice(input: { userId: string, name: string, fingerprint: string }) {
  const [device] = await useDb().insert(userDevices).values({ ...input, pairedAt: new Date(), lastSeenAt: new Date() }).returning()
  return device
}

export async function listApiKeys(userId: string) {
  return useDb().select({ id: apiKeys.id, name: apiKeys.name, keyPrefix: apiKeys.keyPrefix, lastUsedAt: apiKeys.lastUsedAt, createdAt: apiKeys.createdAt }).from(apiKeys).where(eq(apiKeys.userId, userId)).orderBy(desc(apiKeys.createdAt))
}

export async function createApiKey(input: { userId: string, name: string, keyHash: string, keyPrefix: string }) {
  return useDb().insert(apiKeys).values(input).returning({ id: apiKeys.id, name: apiKeys.name, keyPrefix: apiKeys.keyPrefix, createdAt: apiKeys.createdAt })
}

export async function deleteApiKey(userId: string, id: string) {
  return useDb().delete(apiKeys).where(and(eq(apiKeys.id, id), eq(apiKeys.userId, userId))).returning({ id: apiKeys.id })
}

export async function createConversation(input: { userId: string, workspaceId: string, title: string, modelId: string, mode: 'chat' | 'agent', reasoningEffort?: 'low' | 'medium' | 'high' | 'max' }) {
  return useDb().insert(conversations).values(input).returning()
}

export async function updateConversation(userId: string, id: string, input: Record<string, unknown>) {
  return useDb().update(conversations).set({ ...input, updatedAt: new Date() }).where(and(eq(conversations.id, id), eq(conversations.userId, userId))).returning()
}

export async function deleteConversation(userId: string, id: string) {
  return useDb().delete(conversations).where(and(eq(conversations.id, id), eq(conversations.userId, userId))).returning()
}

export async function revokeDevice(userId: string, id: string) {
  return useDb().update(userDevices).set({ revokedAt: new Date() }).where(and(eq(userDevices.id, id), eq(userDevices.userId, userId))).returning()
}
