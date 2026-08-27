import * as v from 'valibot'
import { badRequest } from '#server/core/errors/http'

const schema = v.strictObject({
  url: v.pipe(v.string(), v.trim(), v.minLength(1, 'URL is required'), v.maxLength(2048, 'URL is too long'), v.url('URL must be valid'))
})

export default defineEventHandler(async (event) => {
  await requireUserSession(event)
  const body = await readValidatedBody(event, body => v.parse(schema, body))

  try {
    return await event.context.application.mcp.discoverOAuth(body.url)
  } catch {
    throw badRequest('Unable to inspect MCP authorization metadata')
  }
})
