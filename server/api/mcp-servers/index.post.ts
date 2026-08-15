import * as v from 'valibot'

const createSchema = v.object({
  name: v.string(),
  description: v.optional(v.string(), ''),
  transport: v.string(),
  url: v.optional(v.string()),
  command: v.optional(v.string())
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readValidatedBody(event, body => v.parse(createSchema, body))
  return event.context.application.mcp.createServer(session.user.id, body)
})
