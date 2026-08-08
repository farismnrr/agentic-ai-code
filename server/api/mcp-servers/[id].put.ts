import * as v from 'valibot'
import { updateMcpServer } from '../../utils/mcp-servers'

const updateSchema = v.object({
  name: v.optional(v.string()),
  description: v.optional(v.string()),
  transport: v.optional(v.string()),
  url: v.optional(v.string()),
  command: v.optional(v.string()),
  status: v.optional(v.string()),
  enabled: v.optional(v.boolean()),
  tools: v.optional(v.array(v.any()))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing server ID')

  const body = await readValidatedBody(event, body => v.parse(updateSchema, body))
  return updateMcpServer(session.user.id, id, body)
})
