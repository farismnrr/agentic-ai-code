export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing provider ID')

  const modelIds = await listProviderModelIds(session.user.id, id)
  return modelIds.map(modelId => ({ label: modelId, value: modelId }))
})
