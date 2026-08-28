import { and, desc, eq, isNull } from 'drizzle-orm'
import { useRuntimeConfig } from '#imports'
import { mcpServers, relayActivitySources, workspaces } from '../../database/schema'
import { activityDatabase } from '../database/activity'
import { useDb } from '../database/connection'
import { createMcpClient } from '../mcp/client'

function isOperatorRelayUrl(serverUrlValue: string, siteUrlValue: string) {
  let serverUrl: URL
  let siteUrl: URL
  try {
    serverUrl = new URL(serverUrlValue)
    siteUrl = new URL(siteUrlValue)
  } catch {
    return false
  }
  if (serverUrl.protocol !== 'https:' || siteUrl.protocol !== 'https:') return false
  const siteParts = siteUrl.hostname.toLowerCase().split('.').filter(Boolean)
  if (siteParts.length < 3) return false
  const operatorSuffix = siteParts.slice(1).join('.')
  return serverUrl.hostname.toLowerCase() === `mcp.${operatorSuffix}`
    && serverUrl.pathname === '/mcp'
    && !serverUrl.username
    && !serverUrl.password
    && !serverUrl.search
    && !serverUrl.hash
}

async function bindAllWorkspaces(userId: string, sourceId: string) {
  const owned = await useDb()
    .select({ id: workspaces.id })
    .from(workspaces)
    .where(eq(workspaces.userId, userId))
  for (const workspace of owned) await activityDatabase.bind(userId, sourceId, workspace.id)
}

async function findExistingSource(userId: string, sourceKey: string | undefined, label: string) {
  const db = useDb()
  if (sourceKey) {
    const [matched] = await db
      .select({ id: relayActivitySources.id })
      .from(relayActivitySources)
      .where(and(
        eq(relayActivitySources.userId, userId),
        eq(relayActivitySources.sourceKey, sourceKey),
        isNull(relayActivitySources.revokedAt)
      ))
      .limit(1)
    if (matched) return matched.id
  }
  const [pending] = await db
    .select({ id: relayActivitySources.id })
    .from(relayActivitySources)
    .where(and(
      eq(relayActivitySources.userId, userId),
      eq(relayActivitySources.label, label),
      isNull(relayActivitySources.sourceKey),
      isNull(relayActivitySources.revokedAt)
    ))
    .orderBy(desc(relayActivitySources.createdAt))
    .limit(1)
  return pending?.id
}

/**
 * Enroll and configure the operator-owned relay activity pipeline without
 * exposing its one-time source credential to browser code. The MCP extension
 * is private/non-model-facing and only eligible for the conventional
 * operator-owned `mcp.<same deployment domain>/mcp` resource.
 */
export async function bootstrapRelayActivity(userId: string, serverId: string) {
  const [server] = await useDb()
    .select()
    .from(mcpServers)
    .where(and(eq(mcpServers.id, serverId), eq(mcpServers.userId, userId)))
    .limit(1)
  if (!server || !server.url || server.transport !== 'http' || !server.oauthTokensEncrypted) {
    return { supported: false, configured: false }
  }

  const siteUrl = String(useRuntimeConfig().public.siteUrl ?? '')
  if (!isOperatorRelayUrl(server.url, siteUrl)) return { supported: false, configured: false }

  const client = await createMcpClient({
    userId,
    serverId: server.id,
    name: server.name,
    transport: server.transport,
    url: server.url
  })
  try {
    if (!client.supportsActivityBootstrap?.() || !client.activityStatus || !client.configureActivity) {
      return { supported: false, configured: false }
    }

    const status = await client.activityStatus()
    const label = `MCP relay ${server.id}`.slice(0, 80)
    const existingSourceId = await findExistingSource(userId, status.sourceId, label)
    if (status.configured && existingSourceId) {
      await bindAllWorkspaces(userId, existingSourceId)
      return { supported: true, configured: true }
    }

    const source = await activityDatabase.enroll(userId, { label, kind: 'relay' })
    try {
      await bindAllWorkspaces(userId, source.id)
      const sinkUrl = new URL('/api/activity/ingest', siteUrl).href
      await client.configureActivity({ sinkUrl, sourceToken: source.token })
      return { supported: true, configured: true }
    } catch (error) {
      await activityDatabase.revoke(userId, source.id).catch(() => undefined)
      throw error
    }
  } finally {
    await client.close().catch(() => undefined)
  }
}
