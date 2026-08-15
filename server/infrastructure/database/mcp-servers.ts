import { useDb } from './connection'
import { notFound, conflict, internal } from '#server/core/errors/http'
import { safeDiagnostic } from '#server/core/errors/safe-diagnostic'
import { eq, and } from 'drizzle-orm'
import { mcpServers } from '../../database/schema'
import { isUniqueViolation } from './errors'

import type { McpTool } from '../../../shared/types/chat'

export async function listMcpServers(userId: string) {
  const db = useDb()
  const servers = await db
    .select()
    .from(mcpServers)
    .where(eq(mcpServers.userId, userId))

  return servers.map(s => ({
    id: s.id,
    name: s.name,
    description: s.description,
    transport: s.transport,
    url: s.url,
    command: s.command,
    status: s.status,
    enabled: s.enabled,
    tools: Array.isArray(s.tools) ? s.tools : (typeof s.tools === 'string' ? JSON.parse(s.tools) : s.tools)
  }))
}

export async function createMcpServer(userId: string, body: { name: string, description?: string, transport: string, url?: string, command?: string }) {
  const db = useDb()
  const id = body.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || `server-${Date.now().toString(36)}`

  let server
  try {
    [server] = await db
      .insert(mcpServers)
      .values({
        id: `${id}-${userId}`,
        userId,
        name: body.name,
        description: body.description,
        transport: body.transport,
        url: body.url,
        command: body.command,
        // 'connected' is a claim, not yet a fact — POST /api/mcp-servers/:id/test
        // is what actually verifies it and updates this.
        status: 'disconnected',
        enabled: true,
        tools: []
      })
      .returning()
  } catch (err) {
    if (isUniqueViolation(err)) throw conflict('Server ID already exists')
    throw err
  }

  if (!server) throw internal(safeDiagnostic('Failed to create server'))

  return {
    ...server,
    tools: []
  }
}
export async function updateMcpServer(userId: string, id: string, updates: { name?: string, description?: string, transport?: string, url?: string, command?: string, status?: string, enabled?: boolean, tools?: McpTool[] }) {
  const db = useDb()
  const [updated] = await db
    .update(mcpServers)
    .set({
      ...(updates.name !== undefined && { name: updates.name }),
      ...(updates.description !== undefined && { description: updates.description }),
      ...(updates.transport !== undefined && { transport: updates.transport }),
      ...(updates.url !== undefined && { url: updates.url }),
      ...(updates.status !== undefined && { status: updates.status }),
      ...(updates.enabled !== undefined && { enabled: updates.enabled }),
      ...(updates.tools !== undefined && { tools: updates.tools }),
      ...(updates.command !== undefined && { command: updates.command }),
      updatedAt: new Date()
    })
    .where(and(eq(mcpServers.id, id), eq(mcpServers.userId, userId)))
    .returning()

  if (!updated) throw notFound('Server not found')

  return {
    id: updated.id,
    name: updated.name,
    description: updated.description,
    transport: updated.transport,
    url: updated.url,
    command: updated.command,
    status: updated.status,
    enabled: updated.enabled,
    tools: Array.isArray(updated.tools) ? updated.tools : (typeof updated.tools === 'string' ? JSON.parse(updated.tools) : updated.tools)
  }
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
