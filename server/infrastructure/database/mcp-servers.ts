import { and, eq, lt } from 'drizzle-orm'
import { useDb } from './connection'
import { conflict, internal, notFound } from '#server/core/errors/http'
import { safeDiagnostic } from '#server/core/errors/safe-diagnostic'
import { mcpOauthFlows, mcpServers } from '../../database/schema'
import { isUniqueViolation } from './errors'
import type { McpRemoteConfig, McpStatus, McpTool, McpTransport } from '../../../shared/types/chat'

function parseTools(value: unknown): McpTool[] {
  if (Array.isArray(value)) return value as McpTool[]
  if (typeof value === 'string') return JSON.parse(value) as McpTool[]
  return []
}

function presentServer(server: typeof mcpServers.$inferSelect) {
  const unsupported = server.transport === 'stdio'
  return {
    id: server.id,
    name: server.name,
    description: server.description,
    transport: server.transport as McpTransport,
    url: server.url ?? undefined,
    command: server.command ?? undefined,
    status: (unsupported ? 'error' : server.status) as McpStatus,
    enabled: unsupported ? false : server.enabled,
    tools: unsupported ? [] : parseTools(server.tools)
  }
}

export function mcpServerIdFor(userId: string, name: string) {
  const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'server'
  return `${slug}-${userId}`
}

export async function listMcpServers(userId: string) {
  const db = useDb()
  const servers = await db.select().from(mcpServers).where(eq(mcpServers.userId, userId))
  return servers.map(presentServer)
}

export async function getMcpServer(userId: string, id: string) {
  const db = useDb()
  const [server] = await db.select().from(mcpServers).where(and(eq(mcpServers.id, id), eq(mcpServers.userId, userId))).limit(1)
  return server
}

export async function createMcpServer(userId: string, input: McpRemoteConfig & { id: string, tools: McpTool[] }) {
  const db = useDb()
  let server
  try {
    [server] = await db
      .insert(mcpServers)
      .values({
        id: input.id,
        userId,
        name: input.name,
        description: input.description,
        transport: input.transport,
        url: input.url,
        command: null,
        status: 'connected',
        enabled: true,
        tools: input.tools
      })
      .returning()
  } catch (err) {
    if (isUniqueViolation(err)) throw conflict('Server ID already exists')
    throw err
  }

  if (!server) throw internal(safeDiagnostic('Failed to create server'))
  return presentServer(server)
}

export async function updateMcpServer(
  userId: string,
  id: string,
  updates: {
    name?: string
    description?: string
    transport?: McpRemoteConfig['transport']
    url?: string
    status?: McpStatus
    enabled?: boolean
    tools?: McpTool[]
  }
) {
  const db = useDb()
  const [updated] = await db
    .update(mcpServers)
    .set({
      ...(updates.name !== undefined && { name: updates.name }),
      ...(updates.description !== undefined && { description: updates.description }),
      ...(updates.transport !== undefined && { transport: updates.transport, command: null }),
      ...(updates.url !== undefined && { url: updates.url }),
      ...(updates.status !== undefined && { status: updates.status }),
      ...(updates.enabled !== undefined && { enabled: updates.enabled }),
      ...(updates.tools !== undefined && { tools: updates.tools }),
      updatedAt: new Date()
    })
    .where(and(eq(mcpServers.id, id), eq(mcpServers.userId, userId)))
    .returning()

  if (!updated) throw notFound('Server not found')
  return presentServer(updated)
}

export async function createOAuthPendingMcpServer(userId: string, input: McpRemoteConfig & {
  id: string
  oauthAuthorizationServer: string
  oauthResource: string
  oauthRedirectUri: string
  oauthClientInformationEncrypted: string
  oauthTokensEncrypted: string
}) {
  const db = useDb()
  let server
  try {
    [server] = await db.insert(mcpServers).values({
      id: input.id,
      userId,
      name: input.name,
      description: input.description,
      transport: input.transport,
      url: input.url,
      command: null,
      status: 'connecting',
      enabled: true,
      tools: [],
      oauthAuthorizationServer: input.oauthAuthorizationServer,
      oauthResource: input.oauthResource,
      oauthRedirectUri: input.oauthRedirectUri,
      oauthClientInformationEncrypted: input.oauthClientInformationEncrypted,
      oauthTokensEncrypted: input.oauthTokensEncrypted
    }).returning()
  } catch (err) {
    if (isUniqueViolation(err)) throw conflict('Server ID already exists')
    throw err
  }
  if (!server) throw internal(safeDiagnostic('Failed to create OAuth server'))
  return presentServer(server)
}

export async function getMcpServerOAuthCredentials(userId: string, id: string) {
  const server = await getMcpServer(userId, id)
  if (!server?.oauthAuthorizationServer || !server.oauthResource || !server.oauthRedirectUri || !server.oauthClientInformationEncrypted || !server.oauthTokensEncrypted) return undefined
  return {
    authorizationServer: server.oauthAuthorizationServer,
    resource: server.oauthResource,
    redirectUri: server.oauthRedirectUri,
    clientInformationEncrypted: server.oauthClientInformationEncrypted,
    tokensEncrypted: server.oauthTokensEncrypted
  }
}

export async function updateMcpServerOAuthTokens(userId: string, id: string, oauthTokensEncrypted: string) {
  const db = useDb()
  const [updated] = await db.update(mcpServers)
    .set({ oauthTokensEncrypted, updatedAt: new Date() })
    .where(and(eq(mcpServers.id, id), eq(mcpServers.userId, userId)))
    .returning({ id: mcpServers.id })
  if (!updated) throw notFound('Server not found')
}

export async function createMcpOAuthFlow(input: typeof mcpOauthFlows.$inferInsert) {
  const db = useDb()
  await db.delete(mcpOauthFlows).where(lt(mcpOauthFlows.expiresAt, new Date()))
  await db.insert(mcpOauthFlows).values(input)
}

export async function consumeMcpOAuthFlow(stateHash: string) {
  const db = useDb()
  const [flow] = await db.delete(mcpOauthFlows)
    .where(eq(mcpOauthFlows.stateHash, stateHash))
    .returning()
  return flow
}

export async function deleteMcpServer(userId: string, id: string) {
  const db = useDb()
  const [deleted] = await db
    .delete(mcpServers)
    .where(and(eq(mcpServers.id, id), eq(mcpServers.userId, userId)))
    .returning()

  if (!deleted) throw notFound('Server not found')
  return { ok: true }
}
