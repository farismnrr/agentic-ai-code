export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const serverId = getRouterParam(event, 'id')
  if (!serverId) throw createError({ statusCode: 400, statusMessage: 'MCP server id is required' })
  return event.context.application.mcp.bootstrapActivity(session.user.id, serverId)
})
