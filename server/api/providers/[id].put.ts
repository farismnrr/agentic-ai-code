import * as v from 'valibot'

const bodySchema = v.object({
  name: v.optional(v.string()),
  baseUrl: v.optional(v.string()),
  apiKey: v.optional(v.string()),
  customHeaders: v.optional(v.record(v.string(), v.string())),
  enabled: v.optional(v.boolean())
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing provider ID')

  const body = await readBody(event)
  const parsed = v.safeParse(bodySchema, body)
  if (!parsed.success) {
    throw unprocessable(parsed.issues)
  }

  return updateModelProvider(session.user.id, id, parsed.output)
})
