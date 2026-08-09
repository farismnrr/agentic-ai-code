// Generic titles for statuses that can legitimately arrive from trusted
// framework/library code (h3's own 404, nuxt-auth-utils' 401) without going
// through our RFC 9457 factory. The text is ours, not the error's own
// message — that stays default-deny.
const TRUSTED_TITLES: Record<number, string> = {
  400: 'Bad Request',
  401: 'Unauthorized',
  403: 'Forbidden',
  404: 'Not Found',
  405: 'Method Not Allowed',
  409: 'Conflict'
}

export default defineNitroErrorHandler((error, event) => {
  const data = (error.data && typeof error.data === 'object' ? error.data : {}) as Record<string, unknown>
  const isProblem = data.problem === true

  // An error that went through h3's own createError() (not a raw exception)
  // and carries a status we recognize is safe to pass through as-is — this
  // is what distinguishes nuxt-auth-utils' clean 401 from requireUserSession()
  // from a genuinely unexpected exception. `unhandled`/`fatal` mark the raw
  // ones; see h3's toNodeListener in h3@1.15.11.
  const trustedTitle = !isProblem && !error.unhandled && !error.fatal
    ? TRUSTED_TITLES[error.statusCode]
    : undefined
  const isTrusted = trustedTitle !== undefined

  const status = isProblem ? error.statusCode : isTrusted ? error.statusCode : 500

  const extensionFields = (extData: Record<string, unknown>) => {
    const { problem, type, title, status, detail, ...rest } = extData
    return rest
  }

  const body = isProblem
    ? { type: data.type, title: data.title, status, detail: data.detail, instance: event.path, ...extensionFields(data) }
    : isTrusted
      ? { type: 'about:blank', title: trustedTitle, status, instance: event.path }
      : { type: 'about:blank', title: 'Internal Server Error', status: 500, instance: event.path }

  // Full detail (message asli, stack, error object) HANYA ke server log — tidak pernah ke client.
  if (!isProblem && !isTrusted) logger.error('[unhandled]', error)

  setResponseHeader(event, 'Content-Type', 'application/problem+json')
  if (isProblem && data.retryAfter) {
    setResponseHeader(event, 'Retry-After', Number(data.retryAfter))
  }
  setResponseStatus(event, status)
  return send(event, JSON.stringify(body))
})
