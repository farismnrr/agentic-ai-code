export async function testMcpServer(userId: string, id: string) {
  const { testMcpServer: run } = await import('../infrastructure/mcp/test-server')
  return run(userId, id)
}
