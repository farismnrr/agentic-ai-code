import { useDb } from './connection'
import { and, desc, eq, isNull, lt, ne } from 'drizzle-orm'
import { apiKeys, authSessions, conversations, users, workspaces, userDevices } from '../../database/schema'

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

export async function createConversation(input: { userId: string, workspaceId: string, title: string, modelId: string, mode: 'chat' | 'agent', permissionMode?: 'plan' | 'bypass' | 'manual', reasoningEffort?: 'low' | 'medium' | 'high' | 'max', enabledToolIds?: string[] }) {
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

export async function createAuthSession(input: { id: string, userId: string, secretHash: string }) {
  const [session] = await useDb().insert(authSessions).values(input).returning({
    id: authSessions.id,
    createdAt: authSessions.createdAt,
    lastSeenAt: authSessions.lastSeenAt
  })
  return session
}

export async function listAuthSessions(userId: string) {
  return useDb().select({
    id: authSessions.id,
    createdAt: authSessions.createdAt,
    lastSeenAt: authSessions.lastSeenAt,
    revokedAt: authSessions.revokedAt
  }).from(authSessions).where(and(eq(authSessions.userId, userId), isNull(authSessions.revokedAt))).orderBy(desc(authSessions.createdAt)).limit(25)
}

export async function validateAuthSession(input: { id: string, userId: string, secretHash: string }) {
  const [session] = await useDb().select({ id: authSessions.id }).from(authSessions).where(and(
    eq(authSessions.id, input.id),
    eq(authSessions.userId, input.userId),
    eq(authSessions.secretHash, input.secretHash),
    isNull(authSessions.revokedAt)
  )).limit(1)
  return Boolean(session)
}

export async function touchAuthSession(input: { id: string, userId: string, secretHash: string }) {
  await useDb().update(authSessions).set({ lastSeenAt: new Date() }).where(and(
    eq(authSessions.id, input.id),
    eq(authSessions.userId, input.userId),
    eq(authSessions.secretHash, input.secretHash),
    isNull(authSessions.revokedAt),
    lt(authSessions.lastSeenAt, new Date(Date.now() - 5 * 60 * 1000))
  ))
}

export async function revokeAuthSession(userId: string, id: string) {
  return useDb().update(authSessions).set({ revokedAt: new Date() }).where(and(
    eq(authSessions.id, id),
    eq(authSessions.userId, userId),
    isNull(authSessions.revokedAt)
  )).returning({ id: authSessions.id })
}

export async function revokeOtherAuthSessions(userId: string, currentId: string) {
  return useDb().update(authSessions).set({ revokedAt: new Date() }).where(and(
    eq(authSessions.userId, userId),
    isNull(authSessions.revokedAt),
    ne(authSessions.id, currentId)
  )).returning({ id: authSessions.id })
}

export async function getUserRole(userId: string) {
  const [user] = await useDb().select({ role: users.role }).from(users).where(eq(users.id, userId)).limit(1)
  return user?.role
}
