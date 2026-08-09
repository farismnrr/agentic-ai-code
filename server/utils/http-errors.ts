import type * as v from 'valibot'
import { getLogger } from './otel'

interface ProblemInit {
  status: number
  title: string
  detail?: string
  type?: string // default 'about:blank' per RFC 9457 §4.2
  extra?: Record<string, unknown> // extension members, mis. { errors: [...] } atau retryAfter
}

// Every thrown API error funnels through here, so this is the one place
// that needs instrumenting to get 4xx/5xx responses into Loki — before
// this, only frontend-forwarded logs (server/api/telemetry.post.ts) ever
// reached the logs pipeline; a server-side 502 like a dead upstream
// provider was visible in `docker compose logs` but invisible in Loki.
function problem(init: ProblemInit) {
  const severityNumber = init.status >= 500 ? 17 : 13 // ERROR : WARN, matches telemetry.post.ts's scale
  getLogger('ai-code-server').emit({
    severityNumber,
    severityText: severityNumber === 17 ? 'ERROR' : 'WARN',
    body: `${init.status} ${init.title}${init.detail ? `: ${init.detail}` : ''}`,
    attributes: {
      'service.name': 'ai-code-server',
      'status': init.status,
      'type': init.type ?? 'about:blank',
      ...init.extra
    }
  })

  return createError({
    statusCode: init.status,
    statusMessage: init.title,
    data: {
      problem: true,
      type: init.type ?? 'about:blank',
      title: init.title,
      status: init.status,
      detail: init.detail,
      ...init.extra
    }
  })
}

export const badRequest = (detail?: string) => problem({ status: 400, title: 'Bad Request', detail })
export const unauthorized = (detail?: string) => problem({ status: 401, title: 'Unauthorized', detail })
export const forbidden = (detail?: string) => problem({ status: 403, title: 'Forbidden', detail })
export const notFound = (detail?: string) => problem({ status: 404, title: 'Not Found', detail })
export const conflict = (detail?: string) => problem({ status: 409, title: 'Conflict', detail })
export const gone = (detail?: string) => problem({ status: 410, title: 'Gone', detail })
export const badGateway = (detail?: string) => problem({ status: 502, title: 'Bad Gateway', detail })

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

export const internal = (cause?: unknown) => {
  console.error('[internal]', cause)
  const detail = cause instanceof Error ? cause.message : cause ? String(cause) : undefined
  return problem({ status: 500, title: 'Internal Server Error', detail })
}
