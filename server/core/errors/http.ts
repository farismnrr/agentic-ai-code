import type * as v from 'valibot'
import { classifyRawCause } from './classify'
import { isSafeDiagnostic } from './safe-diagnostic'

// Private diagnostic payload for a thrown problem — carried on the error's
// `data` (server-side only) so the Nitro error handler (server/core/errors/
// index.ts), which already runs inside an event with the request-scoped
// observability capability at `event.context.application.observability`,
// can perform the actual `logger.error`/`logger.warn` write. `server/core`
// must never import `server/infrastructure` directly (Plan 035 P1 layering
// fix) — this keeps error *semantics* here while the logging *capability*
// stays injected at the Nitro composition edge, same pattern already used
// throughout server/api/**.
export interface ProblemLogPayload {
  level: 'warn' | 'error'
  message: string
  cause?: unknown
  attributes: Record<string, unknown>
}

interface ProblemInit {
  status: number
  title: string
  /**
   * Client-visible free text. Only ever serialized to the response body when
   * `status < 500` — 4xx responses may legitimately carry user-actionable
   * detail. For `status >= 500` this field is intentionally ignored on the
   * client body (see `cause` below); passing it anyway is harmless but has
   * no effect on what the caller receives.
   */
  detail?: string
  /**
   * Private diagnostic — an `Error`, upstream/provider text, DB/filesystem
   * detail, or any other free-form failure context. Logged via `logger`
   * (server-side / Loki only) and NEVER placed in the client-visible body,
   * regardless of status code. This is the only channel `internal()`/
   * `badGateway()` should use for dynamic failure context.
   */
  cause?: unknown
  type?: string // default 'about:blank' per RFC 9457 §4.2
  extra?: Record<string, unknown> // extension members, mis. { errors: [...] } atau retryAfter — client-visible only for status < 500
}

// Plan 035 P1 remediation (round 4): a raw `.message` (or a raw non-Error
// thrown value stringified via `String(cause)`) is not a data-classification
// boundary — it can carry request-derived/PII text. Only a
// `SafeDiagnosticError` (a developer opted a composed message in deliberately)
// is safe to surface verbatim; any other Error/
// thrown value is reduced to a bounded static classification instead.
function causeText(cause: unknown): string | undefined {
  if (cause === undefined) return undefined
  if (isSafeDiagnostic(cause)) return cause.message
  return classifyRawCause(cause)
}

// Every thrown API error funnels through here, so this is the one place
// that needs instrumenting to get 4xx/5xx responses into Loki (via
// `logger`, see server/infrastructure/observability/logger.ts) — before this, only
// frontend-forwarded logs (server/api/telemetry.post.ts) ever reached the
// logs pipeline; a server-side 502 like a dead upstream provider was
// visible in `docker compose logs` but invisible in Loki.
//
// Plan 035 Phase 2: for `status >= 500` the client-visible body is ALWAYS a
// generic, static title/detail — `cause` (raw error message, stack, upstream/
// provider text, DB/filesystem text) and `extra` never reach the client for
// 5xx. They are still logged in full, server-side only, via `logger`, so the
// same failure remains fully diagnosable in Loki/console.
export function problem(init: ProblemInit) {
  const isServerError = init.status >= 500
  const logDetail = isServerError ? causeText(init.cause) : init.detail
  const message = `${init.status} ${init.title}${logDetail ? `: ${logDetail}` : ''}`
  const logAttributes = { status: init.status, type: init.type ?? 'about:blank', ...init.extra }
  const logPayload: ProblemLogPayload = {
    level: isServerError ? 'error' : 'warn',
    message,
    cause: isServerError ? init.cause : undefined,
    attributes: logAttributes
  }

  return createError({
    statusCode: init.status,
    statusMessage: init.title,
    data: {
      problem: true,
      type: init.type ?? 'about:blank',
      title: init.title,
      status: init.status,
      detail: isServerError ? undefined : init.detail,
      ...(isServerError ? undefined : init.extra),
      // Private — never spread into the client-visible response body. Only
      // read by the Nitro error handler to perform the actual logger write.
      logPayload
    }
  })
}

export const badRequest = (detail?: string) => problem({ status: 400, title: 'Bad Request', detail })
export const unauthorized = (detail?: string) => problem({ status: 401, title: 'Unauthorized', detail })
export const forbidden = (detail?: string) => problem({ status: 403, title: 'Forbidden', detail })
export const notFound = (detail?: string) => problem({ status: 404, title: 'Not Found', detail })
export const conflict = (detail?: string) => problem({ status: 409, title: 'Conflict', detail })
export const gone = (detail?: string) => problem({ status: 410, title: 'Gone', detail })

// Keep the cause on the private diagnostic channel without composing a new
// SafeDiagnosticError from runtime data. Safe diagnostics are reserved for
// static, developer-authored messages; dynamic wrapping would bypass that
// boundary even when the current classification happens to be bounded.
export const badGateway = (cause?: unknown) =>
  problem({ status: 502, title: 'Bad Gateway', cause })

export function unprocessable(issues: v.BaseIssue<unknown>[]) {
  const errors = issues.map(issue => ({
    path: issue.path?.map(p => p.key).join('.'),
    message: issue.message
  }))
  return problem({ status: 422, title: 'Unprocessable Content', extra: { errors } })
}

export function tooManyRequests(retryAfterSeconds?: number) {
  return problem({ status: 429, title: 'Too Many Requests', extra: { retryAfter: retryAfterSeconds ?? 60 } })
}

export const internal = (cause?: unknown) => problem({ status: 500, title: 'Internal Server Error', cause })
