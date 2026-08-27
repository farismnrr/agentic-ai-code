import * as v from 'valibot'
import { badRequest } from '#server/core/errors/http'
import { McpConnectionError } from '#server/application/mcp'

const remoteConfigSchema = v.strictObject({
  name: v.pipe(v.string(), v.trim(), v.minLength(1, 'Name is required'), v.maxLength(80, 'Name is too long')),
  description: v.optional(v.pipe(v.string(), v.trim(), v.maxLength(280, 'Description is too long')), ''),
  transport: v.picklist(['http', 'sse'] as const),
  url: v.pipe(v.string(), v.trim(), v.minLength(1, 'URL is required'), v.maxLength(2048, 'URL is too long'), v.url('URL must be valid'))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readValidatedBody(event, body => v.parse(remoteConfigSchema, body))

  try {
    return await event.context.application.mcp.scanServer(session.user.id, body)
  } catch (err) {
    if (err instanceof McpConnectionError) throw badRequest('Unable to connect to MCP server')
    throw err
  }
})
