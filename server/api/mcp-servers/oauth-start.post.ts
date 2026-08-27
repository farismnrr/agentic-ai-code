import * as v from 'valibot'
import { badRequest } from '#server/core/errors/http'
import { McpOAuthStartError } from '#server/application/mcp'

const schema = v.strictObject({
  url: v.pipe(v.string(), v.trim(), v.minLength(1), v.maxLength(2048), v.url())
})

export default defineEventHandler(async (event) => {
  await requireUserSession(event)
  const body = await readValidatedBody(event, value => v.parse(schema, value))
  const callbackUrl = new URL('/api/mcp-servers/oauth/callback', getRequestURL(event).origin).href

  try {
    return await event.context.application.mcp.startOAuth(body.url, callbackUrl)
  } catch (error) {
    if (error instanceof McpOAuthStartError) throw badRequest(error.message)
    if (error instanceof Error && /trusted redirect hosts|trusted host|trusted domain/i.test(error.message)) {
      throw badRequest('OAuth client registration rejected the AI Code callback URL')
    }
    throw badRequest('Unable to start MCP OAuth authorization')
  }
})
