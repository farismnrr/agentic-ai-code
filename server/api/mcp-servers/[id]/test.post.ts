/**
 * "Test connection" — connects to a stored server, lists its tools, and
 * persists status/tools on success (or 'error' on failure) so the settings
 * UI reflects reality instead of the optimistic 'connected' default set at
 * creation. Opportunistic refresh only, per plan 012's scope boundary — no
 * polling daemon.
 */
export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing server ID')

  try {
    const result = await event.context.application.mcp.testMcpServer(session.user.id, id)
    if (!result) throw notFound('Server not found')
    return result
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : 'Unknown error connecting to MCP server'

    throw badRequest(message)
  }
})
