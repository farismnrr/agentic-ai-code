import type { ChatCapabilities, McpOAuthDiscovery, McpRemoteConfig, McpServerUpdateInput } from '#shared/types/chat'

export class McpOAuthStartError extends Error {}
export class McpOAuthCallbackError extends Error {}

export class McpConnectionError extends Error {
  constructor() {
    super('MCP connection could not be verified')
    this.name = 'McpConnectionError'
  }
}

export interface McpUseCases {
  testMcpServer: (userId: string, id: string) => Promise<unknown>
  discoverOAuth: (url: string) => Promise<McpOAuthDiscovery>
  startOAuth: (userId: string, input: McpRemoteConfig, redirectUrl: string) => Promise<{ authorizationUrl: string }>
  completeOAuth: (state: string, authorizationCode: string) => Promise<{ id: string }>
  bootstrapActivity: (userId: string, serverId: string) => Promise<{ supported: boolean, configured: boolean }>
  scanServer: (userId: string, input: McpRemoteConfig) => Promise<unknown>
  listServers: (userId: string) => Promise<unknown>
  getChatCapabilities: (userId: string) => Promise<ChatCapabilities>
  createServer: (userId: string, input: McpRemoteConfig) => Promise<unknown>
  updateServer: (userId: string, id: string, input: McpServerUpdateInput) => Promise<unknown>
  deleteServer: (userId: string, id: string) => Promise<unknown>
  listMessages: (userId: string, conversationId: string) => Promise<unknown>
  sendMessage: (userId: string, conversationId: string, text: string) => Promise<unknown>
}
