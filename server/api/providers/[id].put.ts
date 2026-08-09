export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing provider ID')
  const body = await readBody(event)
  return updateModelProvider(session.user.id, id, body)
})
