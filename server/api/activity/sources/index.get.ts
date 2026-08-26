export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  setResponseHeader(event, 'Cache-Control', 'private, no-store')
  return event.context.application.activity.listSources(session.user.id)
})
