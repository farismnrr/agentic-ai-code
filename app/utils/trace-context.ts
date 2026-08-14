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

type RequestLike = string | URL | Request

function toUrlString(input: RequestLike): string {
  if (typeof Request !== 'undefined' && input instanceof Request) return input.url
  return input.toString()
}

/**
 * True only for requests targeting this app's own `/api/**` endpoints.
 * Never returns true for a different origin — this is the single gate that
 * keeps `traceparent` from ever attaching to a third-party fetch (provider
 * APIs, remote MCP servers, OAuth, etc.), per the plan's trust-boundary rule.
 */
export function isSameOriginApiRequest(input: RequestLike): boolean {
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
export function isTelemetryEndpoint(input: RequestLike): boolean {
  if (typeof window === 'undefined') return false
  let url: URL
  try {
    url = new URL(toUrlString(input), window.location.origin)
  } catch {
    return false
  }
  return url.pathname === TELEMETRY_ENDPOINT_PATH
}
