import * as v from 'valibot'
import { badRequest } from '#server/core/errors/http'
import { McpConnectionError } from '#server/application/mcp'

const updateSchema = v.strictObject({
  name: v.optional(v.pipe(v.string(), v.trim(), v.minLength(1, 'Name is required'), v.maxLength(80, 'Name is too long'))),
  description: v.optional(v.pipe(v.string(), v.trim(), v.maxLength(280, 'Description is too long'))),
  transport: v.optional(v.picklist(['http', 'sse'] as const)),
  url: v.optional(v.pipe(v.string(), v.trim(), v.minLength(1, 'URL is required'), v.maxLength(2048, 'URL is too long'), v.url('URL must be valid'))),
  enabled: v.optional(v.boolean())
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing server ID')
  const body = await readValidatedBody(event, body => v.parse(updateSchema, body))

  try {
    return await event.context.application.mcp.updateServer(session.user.id, id, body)
  } catch (err) {
    if (err instanceof McpConnectionError) throw badRequest('Unable to verify MCP server changes')
    throw err
  }
})
