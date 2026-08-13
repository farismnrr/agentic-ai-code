import { deleteWorkspace } from '../../application/features'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing workspace id')

  return deleteWorkspace(session.user.id, id)
})
