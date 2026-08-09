export default defineEventHandler(async (event) => {
  await requireUserSession(event)
  return ['9router', 'gcp_agent_platform']
})
