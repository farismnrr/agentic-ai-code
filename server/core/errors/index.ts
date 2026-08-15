import type { ProblemLogPayload } from './http'
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
  const isServerError = status >= 500
  // Set early in server/plugins/application.server.ts, before any handler
  // runs, so it survives every error path (thrown, raw exception, or
  // trusted framework error) — read back here so 5xx bodies can carry it
  // per Plan 035 Phase 2 requirement #7.
  const requestId = getResponseHeader(event, 'x-request-id') as string | undefined

  const extensionFields = (extData: Record<string, unknown>) => {
    const { type, title, status, detail, problem, logPayload, ...rest } = extData
    return rest
  }

  // `problem()` already strips `detail`/`extra` for `status >= 500` before
  // this handler ever sees it — `data.detail` is guaranteed undefined and
  // `extensionFields(data)` empty for a `isProblem && isServerError` body.
  // Defense in depth: re-strip them here too, so this handler cannot leak
  // caller-supplied free text even if `problem()`'s own sanitization were
  // ever bypassed, and attach `requestId` to every 5xx body.
  const body = isProblem
    ? {
        type: data.type,
        title: data.title,
        status,
        detail: isServerError ? undefined : data.detail,
        instance: event.path,
        ...(isServerError ? { requestId } : extensionFields(data))
      }
    : isTrusted
      ? { type: 'about:blank', title: trustedTitle, status, instance: event.path }
      : { type: 'about:blank', title: 'Internal Server Error', status: 500, instance: event.path, requestId }

  // Full detail (message asli, stack, error object) HANYA ke server log — tidak pernah ke client.
  // `event.context.application.observability.logger` is the same logger
  // instance server/api/** already reaches via event.context.application.*
  // (composed once per request in server/plugins/application.server.ts, well
  // before this handler can run) — so this file never imports
  // server/infrastructure directly, preserving the server/core layering rule.
  const requestLogger = event.context.application?.observability?.logger
  if (!isProblem && !isTrusted) {
    requestLogger?.error('[unhandled]', error)
  } else if (isProblem) {
    const logPayload = data.logPayload as ProblemLogPayload | undefined
    if (logPayload) requestLogger?.[logPayload.level](logPayload.message, logPayload.cause, logPayload.attributes)
  }

  setResponseHeader(event, 'Content-Type', 'application/problem+json')
  if (isProblem && data.retryAfter) {
    setResponseHeader(event, 'Retry-After', Number(data.retryAfter))
  }
  setResponseStatus(event, status)
  return send(event, JSON.stringify(body))
})
