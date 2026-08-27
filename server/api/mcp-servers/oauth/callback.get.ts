import { McpOAuthCallbackError } from '#server/application/mcp'

function queryString(value: unknown, maxLength: number) {
  return typeof value === 'string' && value.length > 0 && value.length <= maxLength ? value : undefined
}

export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'Cache-Control', 'no-store')

  const query = getQuery(event)
  const state = queryString(query.state, 512)
  const authorizationCode = queryString(query.code, 4096)
  const oauthError = queryString(query.error, 256)

  const telemetry = event.context.application.observability.request
  if (oauthError || !state || !authorizationCode) {
    telemetry.event('mcp.oauth.callback', 'denied', { 'mcp.stage': 'callback_input', 'mcp.oauth': true })
    return sendRedirect(event, '/settings/mcp?oauth=error', 302)
  }

  return telemetry.withSpan('mcp.oauth.callback', { 'mcp.stage': 'callback', 'mcp.oauth': true }, async () => {
    try {
      const result = await event.context.application.mcp.completeOAuth(state, authorizationCode)
      telemetry.event('mcp.oauth.callback', 'ok', { 'mcp.stage': 'complete', 'mcp.oauth': true })
      return sendRedirect(event, `/settings/mcp?oauth=success&id=${encodeURIComponent(result.id)}`, 302)
    } catch (error) {
      telemetry.error('mcp.oauth.callback', 'mcp_oauth_callback_failed', error, { 'mcp.stage': 'callback', 'mcp.oauth': true })
      if (error instanceof McpOAuthCallbackError) {
        return sendRedirect(event, '/settings/mcp?oauth=error', 302)
      }
      return sendRedirect(event, '/settings/mcp?oauth=error', 302)
    }
  })
})
