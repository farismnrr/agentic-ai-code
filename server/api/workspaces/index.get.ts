import { listWorkspaces } from '../../application/features'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return listWorkspaces(session.user.id)
})
