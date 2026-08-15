export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return event.context.application.workspaces.list(session.user.id)
})
