import type * as v from 'valibot'

interface ProblemInit {
  status: number
  title: string
  detail?: string
  type?: string // default 'about:blank' per RFC 9457 §4.2
  extra?: Record<string, unknown> // extension members, mis. { errors: [...] } atau retryAfter
}

function problem(init: ProblemInit) {
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
  return problem({ status: 500, title: 'Internal Server Error' })
}
