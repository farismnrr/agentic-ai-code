import type { UIMessage } from 'ai'

/**
 * Messages are the AI SDK's own `UIMessage` — deliberately not a parallel type.
 * Nuxt UI's chat components and `useChat()` both speak this shape, so redefining
 * it here would only create a translation layer that breaks when a backend lands.
 */
export type { UIMessage }

export interface ModelProvider {
  id: string
  type: 'openai_compatible' | 'anthropic_compatible' | 'vertex_ai'
  name: string
  baseUrl: string | null
  // Header values are secrets and never round-trip to the client — only
  // the header names, so the UI can show which custom headers are set.
  customHeaderKeys: string[]
  enabled: boolean
  hasApiKey: boolean
}

export interface ChatModel {
  id: string
  providerId: string
  modelId: string
  label: string
  description: string
  icon: string
  contextWindow: number | null
  maxOutputTokens: number | null
  thinkingEnabled: boolean | null
  thinkingMinTokens: number | null
  thinkingMaxTokens: number | null
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
  annotations?: {
    readOnlyHint?: boolean
    destructiveHint?: boolean
    idempotentHint?: boolean
    openWorldHint?: boolean
  }
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
export type PermissionMode = 'plan' | 'workspace' | 'autonomous' | 'manual'

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
  mode: 'chat' | 'agent'
  permissionMode: PermissionMode
  reasoningEffort?: 'low' | 'medium' | 'high' | 'max'
  /** `McpTool['id']` values the user has switched on for this conversation. */
  enabledToolIds: string[]
  /** `McpTool['id']` → remembered decision, set by "always allow" / "always deny". */
  approvals: Record<string, ApprovalDecision>
  contextSummary?: string | null
  contextSummaryUpToMessageId?: string | null
  contextSummaryUpToCreatedAt?: string | null
  lastMeasuredTokens?: number | null
  lastMeasuredMessageId?: string | null
}
