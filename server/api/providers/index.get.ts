export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return event.context.application.providers.list(session.user.id)
})
