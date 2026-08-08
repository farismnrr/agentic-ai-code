import { listMcpServers } from '../../utils/mcp-servers'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return listMcpServers(session.user.id)
})
