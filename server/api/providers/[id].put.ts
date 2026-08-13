import * as v from 'valibot'

const bodySchema = v.object({
  name: v.optional(v.string()),
  baseUrl: v.optional(v.string()),
  apiKey: v.optional(v.string()),
  // A string value sets/replaces that header; `null` deletes it; a key not
  // present in the object leaves the stored header untouched (so the client
  // can submit only the headers it's actually changing — it never has the
  // secret values of unchanged headers to resend).
  customHeaders: v.optional(v.record(v.string(), v.nullable(v.string()))),
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
