import { Buffer } from 'node:buffer'
import { asMcpTaskEnvelope, fetchWithMcpDeadline, McpRoundTripTimeoutError, mcpRoutingName, taskPollDelayMs } from './task-reliability.ts'
import type { McpClientCallResult, McpClientLike, McpClientTool } from './client'
import { redactSecrets } from '../observability/sanitize.ts'

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
 * relay: `server/discover`, `tools/list`, `tools/call`, task lifecycle, and
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
  }

  supportsActivityBootstrap() {
    return this.activityBootstrapSupported
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
      if (task) {
        // An explicit async call is an acceptance operation. Return the
        // durable task identity immediately so the model can poll the latest
        // state instead of keeping an AI step open until its timeout.
        if (params.arguments?.execution_mode === 'async') {
          return taskProgressResult(task, 'Task accepted asynchronously. Use terminal_job_get with this taskId to read the latest status and output; do not start the command again.')
        }
        return this.awaitTask(task, signal)
      }
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
    let latestTask = initialTask
    let delayMs = taskPollDelayMs(initialTask)
    for (;;) {
      if (signal?.aborted) {
        let current: unknown
        try {
          current = await this.request('tasks/get', { taskId })
        } catch (error) {
          if (error instanceof McpRoundTripTimeoutError) {
            return taskProgressResult(latestTask, 'The status poll timed out. The relay task is still durable; use terminal_job_get with this taskId to resume from the latest known output.')
          }
          throw error
        }
        if (!isJsonRecord(current)) throw new Error('Remote MCP task returned an invalid result')
        if (current.status !== 'working') return taskResult(current)
        return taskProgressResult(current, 'The request ended while the task was working. The relay task continues; use terminal_job_get with this taskId to resume from the latest output. Do not start the command again.')
      }
      let task: unknown
      try {
        task = await this.request('tasks/get', { taskId })
      } catch (error) {
        if (error instanceof McpRoundTripTimeoutError) {
          return taskProgressResult(latestTask, 'The status poll timed out. The relay task is still durable; use terminal_job_get with this taskId to resume from the latest known output.')
        }
        throw error
      }
      if (!isJsonRecord(task)) throw new Error('Remote MCP task returned an invalid result')
      latestTask = task
      const status = typeof task.status === 'string' ? task.status : ''
      if (status === 'input_required') throw new Error('Remote MCP task requires additional input')
      if (status !== 'working') return taskResult(task)
      delayMs = taskPollDelayMs(task, delayMs)
      await new Promise(resolve => setTimeout(resolve, delayMs))
    }
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

  private async request(method: 'server/discover' | 'server/activity_status' | 'server/activity_configure' | 'tools/list' | 'tools/call' | 'resources/list' | 'resources/read' | 'tasks/get' | 'tasks/cancel' | 'agent/subagent_stop', params: Record<string, unknown>) {
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
  if (status === 'failed') return taskProgressResult(task, 'Task execution failed. Use terminal_job_get with this taskId to inspect the retained output.', true)
  if (status === 'cancelled') return taskProgressResult(task, 'Task execution was cancelled. Use terminal_job_get with this taskId to inspect the retained output.', true)
  if (task.executionStatus === 'timed_out') return taskProgressResult(task, 'Task execution timed out. The retained output below is the last known log; do not start the command again unless a new execution is intentional.', true)

  const result = isJsonRecord(task.result) ? task.result : undefined
  if (!result || !Array.isArray(result.content)) {
    return taskProgressResult(task, 'Task returned no result. Use terminal_job_get with this taskId to inspect the retained output.', true)
  }
  return {
    content: result.content,
    ...(typeof result.isError === 'boolean' && { isError: result.isError })
  }
}

function taskProgressResult(task: Record<string, unknown>, message: string, isError = false): McpClientCallResult {
  const progress: Record<string, unknown> = {
    resultType: 'task',
    taskId: typeof task.taskId === 'string' ? task.taskId : '',
    status: typeof task.status === 'string' ? task.status : 'working',
    message
  }
  for (const key of ['executionStatus', 'createdAt', 'lastUpdatedAt', 'pollIntervalMs']) {
    if (task[key] !== undefined) progress[key] = task[key]
  }
  const output = taskOutput(task)
  if (output) progress.output = output
  return {
    content: [{ type: 'text', text: JSON.stringify(progress) }],
    ...(isError && { isError: true })
  }
}

function taskOutput(task: Record<string, unknown>) {
  if (!isJsonRecord(task.output)) return undefined
  const output = task.output
  return {
    stdout: boundedRedactedText(output.stdout),
    stderr: boundedRedactedText(output.stderr),
    omittedBytes: typeof output.omittedBytes === 'number' && Number.isFinite(output.omittedBytes) ? output.omittedBytes : 0,
    exitCode: typeof output.exitCode === 'number' || output.exitCode === null ? output.exitCode : null
  }
}

function boundedRedactedText(value: unknown) {
  if (typeof value !== 'string') return ''
  return redactSecrets(value.slice(0, 65_536))
}
