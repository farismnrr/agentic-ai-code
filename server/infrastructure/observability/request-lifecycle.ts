import type { H3Event } from 'h3'
import { trace } from '@opentelemetry/api'
import { logger } from './logger'
import { sanitizeRoute } from './sanitize'
import { classifyErrorType, classifyRawCause } from '../../core/errors/classify'

const STARTED_AT = Symbol('plan035.request.startedAt')
const RECORDED = Symbol('plan035.request.recorded')
const TRACE_ID = Symbol('plan035.request.traceId')
const SPAN_ID = Symbol('plan035.request.spanId')
const REQUEST_PATH = Symbol('plan035.request.path')
const MAX_DURATION_MS = 60_000
const SAFE_STATIC_ROUTES = new Set(['/api/auth/register'])

type LifecycleContext = Record<PropertyKey, unknown>

type MatchedRouteContext = LifecycleContext & {
  matchedRoute?: { route?: unknown }
}

function lifecycleRoute(event: H3Event): string {
  const context = event.context as MatchedRouteContext | undefined
  const matchedRoute = context?.matchedRoute?.route
  if (typeof matchedRoute === 'string' && matchedRoute.startsWith('/')) {
    return sanitizeRoute(matchedRoute)
  }

  // The request hook can run before Nitro attaches its matched-route context
  // in the production node preset. Preserve exact classification only for
  // static routes explicitly known to contain no attacker-controlled segment;
  // dynamic and unknown paths must continue through the coarse fallback.
  const requestPath = (context?.[REQUEST_PATH] as string | undefined || event.node?.req?.url || event.node?.req?.originalUrl || event.path || '').split('?')[0]
  if (SAFE_STATIC_ROUTES.has(requestPath)) return requestPath

  // A missing match is expected for unmatched/early-failed requests. Keep only
  // a coarse, static family classification; never derive a route from the
  // attacker-controlled path or query string.
  const path = event.path || ''
  if (path.startsWith('/api/')) return '/api/*'
  if (path === '/api') return '/api/*'
  if (path.startsWith('/auth/')) return '/auth/*'
  if (path === '/') return '/'
  return 'unmatched'
}

// Nitro's 'error' hook can fire for errors captured outside a real inbound
// request (e.g. during prerendering, or other internal error-capture paths
// that pass a partial/synthetic event) where `event` itself may be
// missing entirely. Guard against that rather than throwing out of an error
// handler — a missing event means there is no request to correlate, so
// the safe behavior is a no-op, never a crash.
function lifecycleContext(event?: H3Event): LifecycleContext | undefined {
  return event?.context as LifecycleContext | undefined
}

function outcome(status: number): 'ok' | 'client_error' | 'server_error' {
  if (status >= 500) return 'server_error'
  if (status >= 400) return 'client_error'
  return 'ok'
}

export function beginRequestLifecycle(event: H3Event): void {
  const context = lifecycleContext(event)
  if (!context) return
  context[STARTED_AT] = performance.now()
  context[REQUEST_PATH] = event.node?.req?.url || event.node?.req?.originalUrl || event.path || ''
  const spanContext = trace.getActiveSpan()?.spanContext()
  if (spanContext?.traceId) context[TRACE_ID] = spanContext.traceId
  if (spanContext?.spanId) context[SPAN_ID] = spanContext.spanId
}

export function recordRequestLifecycle(event: H3Event | undefined, statusOverride?: number, errorCause?: unknown): void {
  const context = lifecycleContext(event)
  if (!context || !event?.node?.res) return
  if (context[RECORDED] === true) return
  context[RECORDED] = true

  const startedAt = typeof context[STARTED_AT] === 'number' ? context[STARTED_AT] : performance.now()
  const duration = Math.max(0, Math.min(MAX_DURATION_MS, Math.round(performance.now() - startedAt)))
  const status = statusOverride ?? event.node?.res?.statusCode ?? 500
  const attributes: Record<string, unknown> = {
    'event.name': 'http.request.lifecycle',
    'operation': 'http.request.lifecycle',
    'http.request.method': (event.method || 'UNKNOWN').slice(0, 16),
    'route': lifecycleRoute(event),
    'http.response.status_code': status,
    'duration_ms': duration,
    'outcome': outcome(status)
  }
  if (typeof context.requestId === 'string') attributes['request.id'] = context.requestId
  if (typeof context[TRACE_ID] === 'string') attributes.trace_id = context[TRACE_ID]
  if (typeof context[SPAN_ID] === 'string') attributes.span_id = context[SPAN_ID]
  if (errorCause !== undefined) {
    attributes['error.type'] = classifyErrorType(errorCause)
    attributes['error.classification'] = classifyRawCause(errorCause)
  }
  logger.info('http.request.lifecycle', attributes)
}
