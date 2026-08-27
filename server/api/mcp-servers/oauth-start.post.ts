import * as v from 'valibot'
import { badRequest } from '#server/core/errors/http'
import { McpOAuthStartError } from '#server/application/mcp'

const schema = v.strictObject({
  name: v.pipe(v.string(), v.trim(), v.minLength(1), v.maxLength(80)),
  description: v.pipe(v.string(), v.trim(), v.maxLength(280)),
  transport: v.picklist(['http', 'sse'] as const),
  url: v.pipe(v.string(), v.trim(), v.minLength(1), v.maxLength(2048), v.url())
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readValidatedBody(event, value => v.parse(schema, value))
  const callbackUrl = new URL('/api/mcp-servers/oauth/callback', getRequestURL(event).origin).href

  const telemetry = event.context.application.observability.request
  return telemetry.withSpan('mcp.oauth.start', { 'mcp.stage': 'start', 'mcp.transport': body.transport, 'mcp.oauth': true }, async () => {
    try {
      const result = await event.context.application.mcp.startOAuth(session.user.id, body, callbackUrl)
      telemetry.event('mcp.oauth.start', 'ok', { 'mcp.stage': 'redirect_ready', 'mcp.transport': body.transport, 'mcp.oauth': true })
      return result
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      telemetry.error('mcp.oauth.start', 'mcp_oauth_start_failed', error, { 'mcp.stage': 'start', 'mcp.transport': body.transport, 'mcp.oauth': true })
      if (error instanceof McpOAuthStartError) throw badRequest(error.message)
      if (/trusted redirect hosts|trusted host|trusted domain/i.test(errorMessage)) {
        throw badRequest('OAuth client registration rejected the AI Code callback URL')
      }
      throw badRequest('Unable to start MCP OAuth authorization')
    }
  })
})
