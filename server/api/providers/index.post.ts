import * as v from 'valibot'

const bodySchema = v.object({
  type: v.picklist(['9router', 'gcp_agent_platform']),
  name: v.string(),
  baseUrl: v.optional(v.string()),
  apiKey: v.string()
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readValidatedBody(event, body => v.parse(bodySchema, body))
  return createModelProvider(session.user.id, body)
})
