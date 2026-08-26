export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return event.context.application.audit.list(session.user.id)
})
