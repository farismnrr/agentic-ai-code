import * as v from 'valibot'
import { createMcpServer } from '../../application/features'

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
  return createMcpServer(session.user.id, body)
})
