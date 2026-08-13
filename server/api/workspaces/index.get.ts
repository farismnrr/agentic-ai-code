import { listWorkspaces } from '../../infrastructure/composition'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return listWorkspaces(session.user.id)
})
