export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readBody(event)
  const { providerId, ...modelData } = body
  if (!providerId) throw badRequest('Missing providerId')
  return createModel(session.user.id, providerId, modelData)
})
