import { Buffer } from 'node:buffer'
import { context, propagation } from '@opentelemetry/api'
import { Client } from '@modelcontextprotocol/sdk/client/index.js'
import type { OAuthClientProvider } from '@modelcontextprotocol/sdk/client/auth.js'
import type { OAuthClientInformationMixed, OAuthClientMetadata, OAuthTokens } from '@modelcontextprotocol/sdk/shared/auth.js'
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js'
import { SSEClientTransport } from '@modelcontextprotocol/sdk/client/sse.js'
import { assertSafeUrl, createSsrfSafeFetch } from '../security/ssrf-guard'
import { getMcpServerOAuthCredentials, updateMcpServerOAuthTokens } from '../database/mcp-servers'
import { decryptSecret, encryptSecret } from '../security/crypto'
import { asMcpTaskEnvelope, fetchWithMcpDeadline, mcpRoutingName, resolveMcpRequestTimeoutMs, taskPollDelayMs } from './task-reliability'
import { refreshStoredMcpOAuthAccessToken } from './oauth-refresh'

export interface McpClientConfig {
  userId: string
  id?: string
  serverId?: string
  name: string
  transport: string
  url?: string | null
}

type RemoteMcpRuntimeConfig = {
  url?: string
  ownerUserId?: string
  accessToken?: string
  requestTimeoutMs?: number | string
}

const MODERN_MCP_VERSION = '2026-07-28'
const MCP_CLIENT_INFO = { name: 'ai-code', version: '1.0.0' } as const

export type McpClientTool = {
  name: string
  description?: string
  inputSchema: Record<string, unknown>
  annotations?: {
    readOnlyHint?: boolean
    destructiveHint?: boolean
    idempotentHint?: boolean
    openWorldHint?: boolean
  }
}

export type McpClientCallResult = {
  content: unknown[]
  isError?: boolean
  [key: string]: unknown
}

export type McpClientResource = { uri: string, name: string, description?: string, mimeType?: string }
export type McpClientResourceReadResult = { contents: Array<{ uri: string, text?: string, mimeType?: string }>, [key: string]: unknown }

export interface McpClientLike {
  trustedProvenance?: 'first-party-relay' | 'external'
  listTools(): Promise<{ tools: McpClientTool[], [key: string]: unknown }>
  callTool(params: { name: string, arguments?: Record<string, unknown> }, signal?: AbortSignal): Promise<McpClientCallResult>
  listResources?(): Promise<{ resources: McpClientResource[], [key: string]: unknown }>
  readResource?(uri: string): Promise<McpClientResourceReadResult>
  close(): Promise<void>
  subagentStop?(parentSessionId: string, childSessionId: string, status: string): Promise<boolean>
}

function getRemoteMcpRuntimeConfig(): RemoteMcpRuntimeConfig {
  const config = useRuntimeConfig()
  return config.remoteMcp as RemoteMcpRuntimeConfig
}

function parseConfiguredRemoteUrl(raw: string) {
  let url: URL
  try {
    url = new URL(raw)
  } catch {
    throw new Error('Remote MCP runtime configuration has an invalid URL')
  }

  if (url.protocol !== 'https:') {
    throw new Error('Remote MCP runtime configuration must use HTTPS')
  }
  if (url.username || url.password || url.search || url.hash) {
    throw new Error('Remote MCP runtime configuration must not contain credentials, query, or fragment')
  }

  return url
}

/**
 * Returns the private first-party configuration only when both the stored MCP
 * URL and the owning ai-code user match the operator's private runtime config.
 *
 * URL matching alone is not an authorization boundary in this multi-tenant
 * app: another authenticated user can create their own MCP row pointing at the
 * same public URL. Binding the credential to the database row's authoritative
 * `userId` prevents that user from causing Nitro to attach the laptop owner's
 * bearer token to their requests.
 */
function resolveFirstPartyRemote(url: URL, serverConfig: McpClientConfig) {
  const configured = getRemoteMcpRuntimeConfig()
  const configuredUrlRaw = configured.url?.trim()
  if (!configuredUrlRaw) return undefined

  const configuredUrl = parseConfiguredRemoteUrl(configuredUrlRaw)

  // Exact resource identity is intentional: an access token configured for
  // https://host/mcp must never be attached to https://host/other-path.
  if (configuredUrl.href !== url.href) return undefined

  const ownerUserId = configured.ownerUserId?.trim()
  if (!ownerUserId) {
    throw new Error('Remote MCP runtime configuration is missing the owner user id')
  }
  if (serverConfig.userId !== ownerUserId) {
    throw new Error(`Server "${serverConfig.name}" is not available for this user`)
  }

  const accessToken = configured.accessToken?.trim()
  if (!accessToken) {
    throw new Error(`Server "${serverConfig.name}" matches the configured remote MCP resource but no access token is configured`)
  }

  return {
    accessToken,
    requestTimeoutMs: resolveMcpRequestTimeoutMs(configured.requestTimeoutMs)
  }
}

/**
 * Add W3C trace context only for the first-party remote relay. Sending our
 * internal trace identity to arbitrary third-party MCP servers would broaden
 * the telemetry trust boundary unnecessarily.
 */
function withFirstPartyTrace(fetchImpl: typeof fetch): typeof fetch {
  return async (input, init) => {
    const traceHeaders: Record<string, string> = {}
    propagation.inject(context.active(), traceHeaders)

    // Fetch semantics let `init.headers` override headers carried by a Request.
    // Preserve that precedence before injecting trace context.
    const headers = new Headers(input instanceof Request ? input.headers : undefined)
    new Headers(init?.headers).forEach((value, name) => {
      headers.set(name, value)
    })
    for (const [name, value] of Object.entries(traceHeaders)) {
      headers.set(name, value)
    }
    return fetchImpl(input, { ...init, headers })
  }
}

function isJsonRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function encodeMcpHeaderValue(value: string) {
  // The public relay's current tool names are ASCII, but the 2026 transport
  // defines a Base64 sentinel for values that cannot be represented directly
  // as an HTTP header. Keep the adapter correct if a future tool name changes.
  return /^[\x20-\x7e]*$/.test(value)
    ? value
    : `=?base64?${Buffer.from(value, 'utf8').toString('base64')}?=`
}

/**
 * Small, first-party-only MCP 2026 client for the Rust relay.
 *
 * The repository still carries the monolithic MCP SDK v1 for unrelated
 * third-party integrations and the legacy inbound Nuxt MCP endpoint. Rather
 * than weaken the Rust relay or make its modern path depend on a legacy SDK
 * lifecycle, this adapter speaks only the three RPCs ai-code needs against its
 * own relay: `server/discover`, `tools/list`, `tools/call`, and optional
 * explicit resource list/read calls. Resources are never fetched implicitly.
 *
 * It is intentionally not a generic replacement for the MCP SDK. When the
 * repository can migrate the outbound client to `@modelcontextprotocol/client`
 * v2 with an atomic lockfile update + local verification, this class should be
 * deleted in favor of the official modern client with version negotiation.
 */
class ModernHttpMcpClient implements McpClientLike {
  private requestSequence = 0

  constructor(
    private readonly url: URL,
    private readonly accessToken: string,
    private readonly fetchImpl: typeof fetch,
    private readonly requestTimeoutMs: number,
    readonly trustedProvenance: 'first-party-relay' | 'external'
  ) {}

  async connect() {
    const result = await this.request('server/discover', {})
    if (!isJsonRecord(result)
      || !Array.isArray(result.supportedVersions)
      || !result.supportedVersions.includes(MODERN_MCP_VERSION)) {
      throw new Error('Remote MCP server does not advertise the required protocol version')
    }
  }

  async listTools() {
    const result = await this.request('tools/list', {})
    if (!isJsonRecord(result) || !Array.isArray(result.tools)) {
      throw new Error('Remote MCP server returned an invalid tools/list result')
    }

    const tools = result.tools.map((tool) => {
      if (!isJsonRecord(tool)
        || typeof tool.name !== 'string'
        || !isJsonRecord(tool.inputSchema)) {
        throw new Error('Remote MCP server returned an invalid tool definition')
      }
      return {
        ...tool,
        name: tool.name,
        description: typeof tool.description === 'string' ? tool.description : undefined,
        inputSchema: tool.inputSchema,
        annotations: isJsonRecord(tool.annotations)
          ? {
              readOnlyHint: typeof tool.annotations.readOnlyHint === 'boolean' ? tool.annotations.readOnlyHint : undefined,
              destructiveHint: typeof tool.annotations.destructiveHint === 'boolean' ? tool.annotations.destructiveHint : undefined,
              idempotentHint: typeof tool.annotations.idempotentHint === 'boolean' ? tool.annotations.idempotentHint : undefined,
              openWorldHint: typeof tool.annotations.openWorldHint === 'boolean' ? tool.annotations.openWorldHint : undefined
            }
          : undefined
      } satisfies McpClientTool
    })

    return { ...result, tools }
  }

  async listResources() {
    const result = await this.request('resources/list', {})
    if (!isJsonRecord(result) || !Array.isArray(result.resources)) throw new Error('Remote MCP server returned an invalid resources/list result')
    const resources = result.resources.filter(isJsonRecord).flatMap(resource => typeof resource.uri === 'string' && typeof resource.name === 'string'
      ? [{ uri: resource.uri, name: resource.name, description: typeof resource.description === 'string' ? resource.description : undefined, mimeType: typeof resource.mimeType === 'string' ? resource.mimeType : undefined }]
      : [])
    return { ...result, resources }
  }

  async readResource(uri: string) {
    if (uri.length === 0 || uri.length > 4096) throw new Error('Resource URI is invalid')
    const result = await this.request('resources/read', { uri })
    if (!isJsonRecord(result) || !Array.isArray(result.contents)) throw new Error('Remote MCP server returned an invalid resources/read result')
    const contents = result.contents.filter(isJsonRecord).flatMap(content => typeof content.uri === 'string' && (typeof content.text === 'string' || content.text === undefined)
      ? [{ uri: content.uri, text: content.text, mimeType: typeof content.mimeType === 'string' ? content.mimeType : undefined }]
      : [])
    return { ...result, contents }
  }

  async callTool(params: { name: string, arguments?: Record<string, unknown> }, signal?: AbortSignal): Promise<McpClientCallResult> {
    if (signal?.aborted) throw new Error('Remote MCP tool call was cancelled')
    // Do not bind caller cancellation to the initial HTTP round trip. A task
    // id must be received before cancellation can target durable relay work
    // rather than merely abandoning a request whose outcome is unknown.
    const result = await this.request('tools/call', {
      name: params.name,
      arguments: params.arguments ?? {}
    })
    if (!isJsonRecord(result) || !Array.isArray(result.content)) {
      const task = asMcpTaskEnvelope(result)
      if (task) return this.awaitTask(task, signal)
      throw new Error('Remote MCP server returned an invalid tools/call result')
    }
    return {
      ...result,
      content: result.content,
      ...(typeof result.isError === 'boolean' && { isError: result.isError })
    }
  }

  private async awaitTask(initialTask: Record<string, unknown> & { taskId: string }, signal?: AbortSignal): Promise<McpClientCallResult> {
    const taskId = initialTask.taskId
    let delayMs = taskPollDelayMs(initialTask)
    for (;;) {
      if (signal?.aborted) {
        const current = await this.request('tasks/get', { taskId })
        if (!isJsonRecord(current)) throw new Error('Remote MCP task returned an invalid result')
        if (current.status !== 'working') return taskResult(current)
        await this.cancelTask(taskId)
        throw new Error('First-party relay task was cancelled')
      }
      const task = await this.request('tasks/get', { taskId })
      if (!isJsonRecord(task)) throw new Error('Remote MCP task returned an invalid result')
      const status = typeof task.status === 'string' ? task.status : ''
      if (status === 'input_required') throw new Error('Remote MCP task requires additional input')
      if (status !== 'working') return taskResult(task)
      delayMs = taskPollDelayMs(task, delayMs)
      await new Promise(resolve => setTimeout(resolve, delayMs))
    }
  }

  private async cancelTask(taskId: string) {
    await this.request('tasks/cancel', { taskId })
    const deadline = Date.now() + 5000
    let delayMs = 250
    while (Date.now() < deadline) {
      const task = await this.request('tasks/get', { taskId })
      if (isJsonRecord(task) && task.status !== 'working') return
      if (isJsonRecord(task)) delayMs = taskPollDelayMs(task, delayMs)
      const remainingMs = deadline - Date.now()
      if (remainingMs <= 0) break
      await new Promise(resolve => setTimeout(resolve, Math.min(delayMs, remainingMs)))
    }
    throw new Error('First-party relay task cancellation did not settle')
  }

  close() {
    // MCP 2026-07-28 is stateless for this relay. There is no protocol session
    // or background SSE channel to terminate.
    return Promise.resolve()
  }

  async subagentStop(parentSessionId: string, childSessionId: string, status: string) {
    const result = await this.request('agent/subagent_stop', {
      _meta: {
        'io.modelcontextprotocol/agentSession': childSessionId,
        'io.modelcontextprotocol/parentAgentSession': parentSessionId
      },
      status
    })
    return isJsonRecord(result) && result.allowed === true
  }

  private async request(method: 'server/discover' | 'tools/list' | 'tools/call' | 'resources/list' | 'resources/read' | 'tasks/get' | 'tasks/cancel' | 'agent/subagent_stop', params: Record<string, unknown>) {
    const id = `ai-code-${++this.requestSequence}`
    const requestParams = {
      ...params,
      _meta: {
        'io.modelcontextprotocol/protocolVersion': MODERN_MCP_VERSION,
        'io.modelcontextprotocol/clientCapabilities': {
          extensions: { 'io.modelcontextprotocol/tasks': {} }
        },
        'io.modelcontextprotocol/clientInfo': MCP_CLIENT_INFO,
        ...(isJsonRecord(params._meta) ? params._meta : {})
      }
    }
    const headers = new Headers({
      'Accept': 'application/json',
      'Authorization': `Bearer ${this.accessToken}`,
      'Content-Type': 'application/json',
      'MCP-Protocol-Version': MODERN_MCP_VERSION,
      'Mcp-Method': method
    })
    const routingName = mcpRoutingName(method, params)
    if (routingName) headers.set('Mcp-Name', encodeMcpHeaderValue(routingName))

    const response = await fetchWithMcpDeadline(this.fetchImpl, this.url, {
      method: 'POST',
      headers,
      body: JSON.stringify({
        jsonrpc: '2.0',
        id,
        method,
        params: requestParams
      })
    }, this.requestTimeoutMs)

    if (response.status === 401 || response.status === 403) {
      throw Object.assign(new Error('Remote MCP authorization failed'), { code: response.status })
    }
    if (!response.ok) {
      throw Object.assign(new Error('Remote MCP request failed'), { code: response.status })
    }

    let payload: unknown
    try {
      payload = await response.json()
    } catch {
      throw new Error('Remote MCP server returned invalid JSON')
    }
    if (!isJsonRecord(payload)
      || payload.jsonrpc !== '2.0'
      || payload.id !== id) {
      throw new Error('Remote MCP server returned an invalid JSON-RPC response')
    }
    if ('error' in payload) {
      // Do not forward arbitrary remote error text into application logs/UI.
      throw new Error('Remote MCP request returned a protocol error')
    }
    if (!('result' in payload)) {
      throw new Error('Remote MCP response is missing a result')
    }
    return payload.result
  }
}

function taskResult(task: Record<string, unknown>): McpClientCallResult {
  const status = typeof task.status === 'string' ? task.status : ''
  if (status === 'failed') return { content: [{ type: 'text', text: 'Tool execution failed' }], isError: true }
  if (status === 'cancelled') return { content: [{ type: 'text', text: 'Tool execution cancelled' }], isError: true }

  const result = isJsonRecord(task.result) ? task.result : undefined
  if (!result || !Array.isArray(result.content)) {
    return { content: [{ type: 'text', text: 'Tool execution returned no result' }], isError: true }
  }
  return {
    content: result.content,
    ...(typeof result.isError === 'boolean' && { isError: result.isError })
  }
}

class StoredMcpOAuthProvider implements OAuthClientProvider {
  constructor(
    private readonly userId: string,
    private readonly serverId: string,
    private readonly credentials: Awaited<ReturnType<typeof getMcpServerOAuthCredentials>> & {}
  ) {}

  get redirectUrl() {
    return this.credentials.redirectUri
  }

  get clientMetadata(): OAuthClientMetadata {
    return {
      client_name: 'AI Code',
      redirect_uris: [this.credentials.redirectUri],
      grant_types: ['authorization_code', 'refresh_token'],
      response_types: ['code'],
      token_endpoint_auth_method: 'client_secret_basic'
    }
  }

  clientInformation(): OAuthClientInformationMixed {
    return JSON.parse(decryptSecret(this.credentials.clientInformationEncrypted)) as OAuthClientInformationMixed
  }

  tokens(): OAuthTokens {
    return JSON.parse(decryptSecret(this.credentials.tokensEncrypted)) as OAuthTokens
  }

  async saveTokens(tokens: OAuthTokens) {
    const encrypted = encryptSecret(JSON.stringify(tokens))
    await updateMcpServerOAuthTokens(this.userId, this.serverId, encrypted)
    this.credentials.tokensEncrypted = encrypted
  }

  redirectToAuthorization() {
    throw new Error('Stored MCP OAuth session requires interactive reauthorization')
  }

  saveCodeVerifier() {
    throw new Error('Stored MCP OAuth session cannot start a new authorization flow')
  }

  codeVerifier(): string {
    throw new Error('Stored MCP OAuth session has no pending PKCE verifier')
  }
}

async function storedOAuthProvider(serverConfig: McpClientConfig) {
  const serverId = serverConfig.serverId ?? serverConfig.id
  if (!serverId) return undefined
  const credentials = await getMcpServerOAuthCredentials(serverConfig.userId, serverId)
  if (!credentials) return undefined
  return new StoredMcpOAuthProvider(serverConfig.userId, serverId, credentials)
}

/**
 * Connects to a stored MCP server, per request — no pooling, no reconnect
 * logic (see plan 012's "Scope boundary" decision). Callers must `close()`
 * when done.
 *
 * `stdio` transport is deliberately unsupported here: it would spawn
 * `mcpServers.command` — a value any authenticated user can set — as a
 * server-side child process. Rows with `transport: 'stdio'` fail closed.
 *
 * Generic HTTP/SSE rows still use the SDK v1 client for existing third-party
 * integrations. Plan 036's exact first-party remote resource instead uses the
 * strict MCP 2026 adapter above, with an externally-issued private Bearer token
 * and the same redirect/DNS SSRF guard used by provider traffic.
 */
export async function createMcpClient(serverConfig: McpClientConfig): Promise<McpClientLike> {
  if (serverConfig.transport === 'stdio') {
    throw new Error(`Server "${serverConfig.name}" uses the stdio transport, which is not enabled for outbound connections (see server/infrastructure/mcp/client.ts)`)
  }

  if (!serverConfig.url) {
    throw new Error(`Server "${serverConfig.name}" is missing a url for the ${serverConfig.transport} transport`)
  }

  const url = new URL(serverConfig.url)
  await assertSafeUrl(url, `Server "${serverConfig.name}"`)
  const firstParty = resolveFirstPartyRemote(url, serverConfig)

  if (firstParty) {
    if (serverConfig.transport !== 'http') {
      throw new Error('The configured first-party remote MCP resource must use the http transport')
    }
    const guardedFetch = createSsrfSafeFetch(`Server "${serverConfig.name}"`)
    const client = new ModernHttpMcpClient(
      url,
      firstParty.accessToken,
      withFirstPartyTrace(guardedFetch),
      firstParty.requestTimeoutMs,
      'first-party-relay'
    )
    await client.connect()
    return client
  }

  const client = new Client({ name: MCP_CLIENT_INFO.name, version: MCP_CLIENT_INFO.version }, { capabilities: {} })
  const authProvider = await storedOAuthProvider(serverConfig)
  const guardedFetch = createSsrfSafeFetch(`Server "${serverConfig.name}"`)
  const transport = serverConfig.transport === 'sse'
    ? new SSEClientTransport(url, { authProvider })
    : new StreamableHTTPClientTransport(url, {
        authProvider,
        fetch: guardedFetch
      })

  try {
    await client.connect(transport)
    return Object.assign(client as unknown as McpClientLike, { trustedProvenance: 'external' as const })
  } catch (error) {
    await client.close().catch(() => undefined)

    const canTryModernOAuthHttp = serverConfig.transport === 'http' && Boolean(authProvider)
    if (!canTryModernOAuthHttp || !authProvider) throw error

    const tokens = await Promise.resolve(authProvider.tokens())
    if (!tokens?.access_token) throw error

    const modernClient = new ModernHttpMcpClient(
      url,
      tokens.access_token,
      guardedFetch,
      resolveMcpRequestTimeoutMs(undefined),
      'external'
    )
    try {
      await modernClient.connect()
      return modernClient
    } catch (modernError) {
      await modernClient.close().catch(() => undefined)
      const modernStatus = modernError instanceof Error && 'code' in modernError
        ? Number((modernError as Error & { code?: unknown }).code)
        : undefined
      const serverId = serverConfig.serverId ?? serverConfig.id
      if ((modernStatus !== 401 && modernStatus !== 403) || !serverId) throw modernError

      const refreshedAccessToken = await refreshStoredMcpOAuthAccessToken(
        serverConfig.userId,
        serverId,
        url,
        guardedFetch
      )
      if (!refreshedAccessToken) throw modernError

      const refreshedClient = new ModernHttpMcpClient(
        url,
        refreshedAccessToken,
        guardedFetch,
        resolveMcpRequestTimeoutMs(undefined),
        'external'
      )
      await refreshedClient.connect()
      return refreshedClient
    }
  }
}

/**
 * Creates the server-configured first-party relay client for Nuxt-owned
 * server work. This path is intentionally independent of the MCP database
 * rows used by user-selected third-party servers: the relay URL and bearer
 * token come only from private runtime configuration, and the URL is still
 * checked by the same SSRF policy before any request is sent.
 */
export async function createConfiguredFirstPartyRelayClient(): Promise<McpClientLike | undefined> {
  const configured = getRemoteMcpRuntimeConfig()
  const configuredUrlRaw = configured.url?.trim()
  const accessToken = configured.accessToken?.trim()

  if (!configuredUrlRaw && !accessToken) return undefined
  if (!configuredUrlRaw || !accessToken) {
    throw new Error('Remote MCP runtime configuration requires both URL and access token')
  }

  const url = parseConfiguredRemoteUrl(configuredUrlRaw)
  const guardedFetch = createSsrfSafeFetch('Configured first-party remote MCP')
  const client = new ModernHttpMcpClient(
    url,
    accessToken,
    withFirstPartyTrace(guardedFetch),
    resolveMcpRequestTimeoutMs(configured.requestTimeoutMs),
    'first-party-relay'
  )
  await client.connect()
  return client
}
