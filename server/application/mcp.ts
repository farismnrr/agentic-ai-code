export interface McpCapabilities { testMcpServer: (userId: string, id: string) => Promise<unknown> }
export const createMcp = <T extends McpCapabilities>(capabilities: T) => capabilities
