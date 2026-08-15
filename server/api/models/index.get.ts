export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return event.context.application.models.list(session.user.id)
})
