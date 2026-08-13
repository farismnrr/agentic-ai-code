import { deleteModelProvider } from '../../application/features'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing provider ID')
  return deleteModelProvider(session.user.id, id)
})
