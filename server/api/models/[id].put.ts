export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing model ID')
  const body = await readBody(event)
  return updateModel(session.user.id, id, body)
})
