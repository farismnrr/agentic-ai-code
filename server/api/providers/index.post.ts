export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readBody(event)
  return createModelProvider(session.user.id, body)
})
