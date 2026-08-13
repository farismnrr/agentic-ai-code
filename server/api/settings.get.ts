export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return event.context.application.settings.read(session.user.id, { name: session.user.name, email: session.user.email })
})
