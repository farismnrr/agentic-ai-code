export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing server ID')
  return event.context.application.mcp.deleteServer(session.user.id, id)
})
