import { deleteMcpServer } from '../../utils/mcp-servers'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing server ID')
  return deleteMcpServer(session.user.id, id)
})
