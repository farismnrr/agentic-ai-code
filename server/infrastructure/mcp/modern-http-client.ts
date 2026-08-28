import { Buffer } from 'node:buffer'
import { asMcpTaskEnvelope, fetchWithMcpDeadline, mcpRoutingName, taskPollDelayMs } from './task-reliability.ts'
import type { McpClientCallResult, McpClientLike, McpClientTool, TaskCompletionResult } from './client'

const MODERN_MCP_VERSION = '2026-07-28'
const MCP_CLIENT_INFO = { name: 'ai-code', version: '1.0.0' } as const

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
 * lifecycle, this adapter speaks only the RPCs ai-code needs against its own
 * relay: `server/discover`, `tools/list`, `tools/call`, task completion, and
 * optional explicit resource list/read calls. Resources are never fetched
 * implicitly.
 *
 * It is intentionally not a generic replacement for the MCP SDK. When the
 * repository can migrate the outbound client to `@modelcontextprotocol/client`
 * v2 with an atomic lockfile update + local verification, this class should be
 * deleted in favor of the official modern client with version negotiation.
 */
export class ModernHttpMcpClient implements McpClientLike {
  private requestSequence = 0
  private activityBootstrapSupported = false
  private taskCompletionSupported = false
  private readonly url: URL
  private readonly accessToken: string
  private readonly fetchImpl: typeof fetch
  private readonly requestTimeoutMs: number
  readonly trustedProvenance: 'first-party-relay' | 'external'

  constructor(
    url: URL,
    accessToken: string,
    fetchImpl: typeof fetch,
    requestTimeoutMs: number,
    trustedProvenance: 'first-party-relay' | 'external'
  ) {
    this.url = url
    this.accessToken = accessToken
    this.fetchImpl = fetchImpl
    this.requestTimeoutMs = requestTimeoutMs
    this.trustedProvenance = trustedProvenance
  }

  async connect() {
    const result = await this.request('server/discover', {})
    if (!isJsonRecord(result)
      || !Array.isArray(result.supportedVersions)
      || !result.supportedVersions.includes(MODERN_MCP_VERSION)) {
      throw new Error('Remote MCP server does not advertise the required protocol version')
    }
    const capabilities = isJsonRecord(result.capabilities) ? result.capabilities : undefined
    const extensions = capabilities && isJsonRecord(capabilities.extensions) ? capabilities.extensions : undefined
    const bootstrap = extensions?.['io.masihawam/activity-bootstrap']
    this.activityBootstrapSupported = isJsonRecord(bootstrap) && bootstrap.version === '1'
    const taskCompletion = extensions?.['io.masihawam/task-completion-notifications']
    this.taskCompletionSupported = isJsonRecord(taskCompletion)
      && taskCompletion.version === '1'
      && taskCompletion.method === 'server/task_completed'
  }

  supportsActivityBootstrap() {
    return this.activityBootstrapSupported
  }

  supportsTaskCompletion() {
    return this.taskCompletionSupported
  }

  async taskCompleted(input: { taskId: string, title: string, summary: string, completedAt?: string, resultUrl?: string }): Promise<TaskCompletionResult> {
    if (!this.taskCompletionSupported) throw new Error('Remote MCP server does not support task completion notifications')
    const result = await this.request('server/task_completed', input)
    if (!isJsonRecord(result) || !['queued', 'already_sent', 'disabled'].includes(result.status as string)) {
      throw new Error('Remote MCP task completion result is invalid')
    }
    return { status: result.status as TaskCompletionResult['status'] }
  }

  async activityStatus() {
    if (!this.activityBootstrapSupported) return { configured: false }
    const result = await this.request('server/activity_status', {})
    if (!isJsonRecord(result) || typeof result.configured !== 'boolean') {
      throw new Error('Remote MCP activity status is invalid')
    }
    return {
      configured: result.configured,
      sourceId: typeof result.sourceId === 'string' ? result.sourceId : undefined
    }
  }

  async configureActivity(input: { sinkUrl: string, sourceToken: string }) {
    if (!this.activityBootstrapSupported) throw new Error('Remote MCP server does not support activity bootstrap')
    const result = await this.request('server/activity_configure', input)
    if (!isJsonRecord(result) || result.configured !== true) {
      throw new Error('Remote MCP activity bootstrap failed')
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

  private async request(method: 'server/discover' | 'server/activity_status' | 'server/activity_configure' | 'server/task_completed' | 'tools/list' | 'tools/call' | 'resources/list' | 'resources/read' | 'tasks/get' | 'tasks/cancel' | 'agent/subagent_stop', params: Record<string, unknown>) {
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
