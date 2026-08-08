import type { UIMessage } from 'ai'

/**
 * Messages are the AI SDK's own `UIMessage` — deliberately not a parallel type.
 * Nuxt UI's chat components and `useChat()` both speak this shape, so redefining
 * it here would only create a translation layer that breaks when a backend lands.
 */
export type { UIMessage }

export interface ChatModel {
  id: string
  label: string
  description: string
  icon: string
  supportsReasoning?: boolean
}

export type McpTransport = 'http' | 'sse' | 'stdio'

export type McpStatus = 'connected' | 'connecting' | 'disconnected' | 'error'

export interface McpTool {
  /** Fully qualified as `<serverId>.<name>` — unique across servers. */
  id: string
  serverId: string
  name: string
  description: string
  /** Example arguments, used by the mock to render a realistic tool call. */
  sampleInput: Record<string, unknown>
}

export interface McpServer {
  id: string
  name: string
  description: string
  transport: McpTransport
  /** Set for `http`/`sse` transports. */
  url?: string
  /** Set for the `stdio` transport. */
  command?: string
  status: McpStatus
  enabled: boolean
  tools: McpTool[]
}

/** Remembered answer to an approval prompt, scoped to one conversation. */
export type ApprovalDecision = 'always' | 'never'

export interface Workspace {
  id: string
  name: string
  path: string
  pathConfirmed: boolean
  createdAt: number
  updatedAt: number
}

export interface Conversation {
  id: string
  workspaceId: string
  title: string
  createdAt: number
  updatedAt: number
  messages: UIMessage[]
  modelId: string
  /** `McpTool['id']` values the user has switched on for this conversation. */
  enabledToolIds: string[]
  /** `McpTool['id']` → remembered decision, set by "always allow" / "always deny". */
  approvals: Record<string, ApprovalDecision>
}
