export interface McpUseCases {
  testMcpServer: (userId: string, id: string) => Promise<unknown>
  listServers: (userId: string) => Promise<unknown>
  createServer: (userId: string, input: unknown) => Promise<unknown>
  updateServer: (userId: string, id: string, input: unknown) => Promise<unknown>
  deleteServer: (userId: string, id: string) => Promise<unknown>
  listMessages: (userId: string, conversationId: string) => Promise<unknown>
  sendMessage: (userId: string, conversationId: string, text: string) => Promise<unknown>
}
