// Plan 035 Phase 4 — hand-rolled W3C trace-context helpers for the browser.
// Deliberately no OTel Web SDK dependency (plan explicitly forbids exposing
// collectors/opaque payloads to the browser, see
// .agents/plans/035-end-to-end-observability-and-secure-telemetry.md
// "Frontend trace continuity"). IDs are generated with
// crypto.getRandomValues, format matches the standard
// `00-<32 hex trace id>-<16 hex span id>-<flags>` traceparent grammar.

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('')
}

function randomHex(byteLength: number): string {
  const bytes = new Uint8Array(byteLength)
  crypto.getRandomValues(bytes)
  return toHex(bytes)
}

/** 32 hex chars (16 bytes) per the W3C trace-context spec. */
export function generateTraceId(): string {
  return randomHex(16)
}

/** 16 hex chars (8 bytes) per the W3C trace-context spec. */
export function generateSpanId(): string {
  return randomHex(8)
}

/** Builds a standards-compliant sampled traceparent header value. */
export function buildTraceparent(traceId: string, spanId: string): string {
  return `00-${traceId}-${spanId}-01`
}

const TELEMETRY_ENDPOINT_PATH = '/api/telemetry'

function toUrlString(input: unknown): string {
  if (typeof Request !== 'undefined' && input instanceof Request) return input.url
  return String(input)
}

/**
 * True only for requests targeting this app's own `/api/**` endpoints.
 * Never returns true for a different origin — this is the single gate that
 * keeps `traceparent` from ever attaching to a third-party fetch (provider
 * APIs, remote MCP servers, OAuth, etc.), per the plan's trust-boundary rule.
 */
export function isSameOriginApiRequest(input: unknown): boolean {
  if (typeof window === 'undefined') return false
  let url: URL
  try {
    url = new URL(toUrlString(input), window.location.origin)
  } catch {
    return false
  }
  return url.origin === window.location.origin && url.pathname.startsWith('/api/')
}

/** True when the request targets `/api/telemetry` itself — never self-instrumented. */
export function isTelemetryEndpoint(input: unknown): boolean {
  if (typeof window === 'undefined') return false
  let url: URL
  try {
    url = new URL(toUrlString(input), window.location.origin)
  } catch {
    return false
  }
  return url.pathname === TELEMETRY_ENDPOINT_PATH
}

/**
 * The single reusable traced-fetch primitive (Plan 035 P1 — "actual browser
 * chat trace continuity"). Matches the native `fetch` signature so it can be
 * plugged into ANY fetch-consuming call site — both `globalThis.$fetch`
 * (`trace-context.client.ts`, via ofetch's `create(defaults, { fetch })`
 * underlying-fetch hook) and the AI SDK's `DefaultChatTransport({ fetch })`
 * option (`chat-transport.ts`) — without duplicating trace-generation or
 * telemetry-recording logic between them.
 *
 * Same security rules as before, enforced in one place:
 *  - `traceparent` is injected ONLY for `isSameOriginApiRequest()` requests —
 *    never for a third-party origin fetch, no matter which call site uses
 *    this function.
 *  - no `baggage` header, ever.
 *  - the response's `x-request-id` header is captured and correlated with
 *    the trace ID via `useTelemetry().logEvent`, the same Phase 4 event path
 *    other requests already use.
 */
export function createTracedFetch(
  telemetry: Pick<ReturnType<typeof useTelemetry>, 'logEvent'>,
  baseFetch: typeof fetch = globalThis.fetch
): typeof fetch {
  return async (input: Parameters<typeof fetch>[0], init?: Parameters<typeof fetch>[1]) => {
    if (!isSameOriginApiRequest(input)) return baseFetch(input, init)

    const traceId = generateTraceId()
    const spanId = generateSpanId()
    const headers = new Headers(init?.headers)
    headers.set('traceparent', buildTraceparent(traceId, spanId))

    let response: Response
    try {
      response = await baseFetch(input, { ...init, headers })
    } catch (error) {
      if (!isTelemetryEndpoint(input)) {
        telemetry.logEvent('error', 'api.request.error', 'API request failed', {
          'trace.id': traceId,
          'outcome': 'error'
        })
      }
      throw error
    }

    if (!isTelemetryEndpoint(input)) {
      const requestId = response.headers.get('x-request-id') ?? undefined
      const ok = response.status < 400
      telemetry.logEvent(
        ok ? 'info' : 'warn',
        ok ? 'api.request.success' : 'api.request.error',
        `API request ${ok ? 'succeeded' : 'failed'}`,
        {
          'http.response.status_code': response.status,
          'trace.id': traceId,
          'request.id': requestId,
          'outcome': ok ? 'success' : 'error'
        }
      )
    }

    return response
  }
}
