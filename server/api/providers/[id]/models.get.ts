export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing provider ID')

  const modelIds = await event.context.application.providers.discoverModels(session.user.id, id)
  return modelIds.map((modelId: string) => ({ label: modelId, value: modelId }))
})
