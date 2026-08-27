import { logger } from '../observability/logger'
import { notFound } from '#server/core/errors/http'
import { McpConnectionError } from '#server/application/mcp'
import { discoverOAuthServerInfo } from '@modelcontextprotocol/sdk/client/auth.js'
import { createMcpClient } from './client'
import { createMcpServer, getMcpServer, mcpServerIdFor, updateMcpServer } from '../database/mcp-servers'
import { assertSafeUrl, createSsrfSafeFetch } from '../security/ssrf-guard'
import { withInfrastructureSpan } from '../observability/span'
import type { McpDiscoveredTool, McpOAuthDiscovery, McpRemoteConfig, McpRemoteTransport, McpScanResult, McpServerUpdateInput, McpTool } from '#shared/types/chat'

function isRemoteTransport(value: string): value is McpRemoteTransport {
  return value === 'http' || value === 'sse'
}

async function discoverTools(userId: string, config: McpRemoteConfig): Promise<McpDiscoveredTool[]> {
  let client
  try {
    client = await createMcpClient({
      userId,
      name: config.name,
      transport: config.transport,
      url: config.url
    })
    const listed = await client.listTools()
    return listed.tools.map(tool => ({
      name: tool.name,
      description: tool.description ?? '',
      annotations: tool.annotations
    }))
  } catch {
    throw new McpConnectionError()
  } finally {
    if (client) {
      await client.close().catch((err: unknown) => logger.error('[mcp discovery] error closing client', err))
    }
  }
}

async function discoverStoredTools(userId: string, serverId: string, config: McpRemoteConfig) {
  const telemetry = { 'mcp.transport': config.transport, 'external.system': 'mcp_server' }
  const client = await withInfrastructureSpan(
    'mcp.client.connect',
    { ...telemetry, 'mcp.stage': 'connect' },
    () => createMcpClient({
      userId,
      serverId,
      name: config.name,
      transport: config.transport,
      url: config.url
    })
  ).catch(() => {
    throw new McpConnectionError()
  })

  try {
    const listed = await withInfrastructureSpan(
      'mcp.client.list_tools',
      { ...telemetry, 'mcp.stage': 'list_tools', 'mcp.method': 'tools/list' },
      () => client.listTools()
    )
    const trustedProvenance = client.trustedProvenance ?? 'external'
    return listed.tools.map(tool => ({
      id: `${serverId}.${tool.name}`,
      serverId,
      name: tool.name,
      description: tool.description ?? '',
      sampleInput: {},
      annotations: tool.annotations,
      trustedProvenance
    } satisfies McpTool))
  } catch {
    throw new McpConnectionError()
  } finally {
    await client.close().catch((err: unknown) => logger.error('[mcp discovery] error closing client', err))
  }
}

function boundedUrl(value: unknown) {
  return typeof value === 'string' && value.length <= 2048 ? value : undefined
}

function boundedStringList(value: unknown, maxItems = 64) {
  if (!Array.isArray(value)) return []
  return value
    .filter((item): item is string => typeof item === 'string' && item.length > 0 && item.length <= 256)
    .slice(0, maxItems)
}

export async function discoverMcpOAuth(urlValue: string): Promise<McpOAuthDiscovery> {
  const url = new URL(urlValue)
  await assertSafeUrl(url, 'MCP OAuth discovery')
  const fetchFn = createSsrfSafeFetch('MCP OAuth discovery')

  try {
    const info = await discoverOAuthServerInfo(url, { fetchFn })
    const metadata = info.authorizationServerMetadata as Record<string, unknown> | undefined
    const resourceMetadata = info.resourceMetadata as Record<string, unknown> | undefined
    const scopes = boundedStringList(resourceMetadata?.scopes_supported ?? metadata?.scopes_supported)
    const oidcEnabled = scopes.includes('openid')
    const authorizationServer = boundedUrl(info.authorizationServerUrl)
    const issuer = boundedUrl(metadata?.issuer) ?? authorizationServer
    const registrationMethods: Array<'dcr' | 'cimd'> = []
    if (boundedUrl(metadata?.registration_endpoint)) registrationMethods.push('dcr')
    if (metadata?.client_id_metadata_document_supported === true) registrationMethods.push('cimd')

    return {
      available: Boolean(boundedUrl(metadata?.authorization_endpoint) && boundedUrl(metadata?.token_endpoint)),
      authorizationUrl: boundedUrl(metadata?.authorization_endpoint),
      tokenUrl: boundedUrl(metadata?.token_endpoint),
      registrationUrl: boundedUrl(metadata?.registration_endpoint),
      authorizationServer: issuer,
      resource: boundedUrl(resourceMetadata?.resource) ?? url.href,
      scopes,
      oidcEnabled,
      oidcConfigurationUrl: oidcEnabled && issuer ? new URL('.well-known/openid-configuration', issuer.endsWith('/') ? issuer : `${issuer}/`).href : undefined,
      oidcUserinfoEndpoint: boundedUrl(metadata?.userinfo_endpoint),
      oidcScopesSupported: boundedStringList(metadata?.scopes_supported),
      registrationMethods
    }
  } catch {
    return {
      available: false,
      scopes: [],
      oidcEnabled: false,
      oidcScopesSupported: [],
      registrationMethods: []
    }
  }
}

export async function scanMcpServer(userId: string, config: McpRemoteConfig): Promise<McpScanResult> {
  const tools = await discoverTools(userId, config)
  return { transport: config.transport, tools }
}

export async function createVerifiedMcpServer(userId: string, config: McpRemoteConfig) {
  const id = mcpServerIdFor(userId, config.name)
  const tools = await discoverStoredTools(userId, id, config)
  return createMcpServer(userId, { ...config, id, tools })
}

export async function testMcpServer(userId: string, id: string) {
  const server = await getMcpServer(userId, id)
  if (!server) return null

  if (!isRemoteTransport(server.transport) || !server.url) {
    await updateMcpServer(userId, id, { status: 'error', enabled: false, tools: [] })
    throw new McpConnectionError()
  }

  try {
    const tools = await discoverStoredTools(userId, id, {
      name: server.name,
      description: server.description,
      transport: server.transport,
      url: server.url
    })
    const updated = await updateMcpServer(userId, id, { status: 'connected', tools })
    return { id: updated.id, status: updated.status, tools: updated.tools }
  } catch (err) {
    await updateMcpServer(userId, id, { status: 'error', tools: [] })
    throw err
  }
}

export async function updateVerifiedMcpServer(userId: string, id: string, input: McpServerUpdateInput) {
  const server = await getMcpServer(userId, id)
  if (!server) throw notFound('Server not found')

  if (!isRemoteTransport(server.transport) || !server.url) {
    if (input.enabled === true || input.transport !== undefined || input.url !== undefined) {
      throw new McpConnectionError()
    }
    return updateMcpServer(userId, id, {
      ...(input.name !== undefined && { name: input.name }),
      ...(input.description !== undefined && { description: input.description }),
      enabled: false,
      status: 'error',
      tools: []
    })
  }

  const next: McpRemoteConfig = {
    name: input.name ?? server.name,
    description: input.description ?? server.description,
    transport: input.transport ?? server.transport,
    url: input.url ?? server.url
  }
  const connectionChanged = next.transport !== server.transport || next.url !== server.url

  if (!connectionChanged) {
    return updateMcpServer(userId, id, {
      ...(input.name !== undefined && { name: input.name }),
      ...(input.description !== undefined && { description: input.description }),
      ...(input.enabled !== undefined && { enabled: input.enabled })
    })
  }

  const tools = await discoverStoredTools(userId, id, next)
  return updateMcpServer(userId, id, {
    ...next,
    ...(input.enabled !== undefined && { enabled: input.enabled }),
    status: 'connected',
    tools
  })
}
