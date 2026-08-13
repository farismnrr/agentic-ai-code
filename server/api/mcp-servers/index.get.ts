export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return event.context.application.mcp.listServers(session.user.id)
})
