import { listMcpServers } from '../../application/features'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return listMcpServers(session.user.id)
})
