export default defineEventHandler(async (event) => {
  const { user } = await requireUserSession(event)
  return event.context.application.account.listApiKeys(user.id)
})
