// Application/core-safe observability contract (Plan 035 Phase 1). This type
// is the only observability surface application/core code may depend on —
// no @opentelemetry/*, Drizzle/schema, H3/Nitro event types, or provider/AI/
// MCP SDK imports belong here or in any module that imports this file from
// server/application or server/core. The concrete implementation (OTel spans,
// Loki-bound logger) lives in server/infrastructure/observability/** and is
// composed into `event.context.application.observability.request` at the
// existing Nitro composition edge (server/infrastructure/composition/application.ts).
export interface RequestTelemetryContext {
  /** Server-generated identifier for this inbound request. Safe to return to the client. */
  requestId: string
  /** Read-only correlation value for the active distributed trace, when tracing is enabled. */
  traceId?: string
  /** Read-only correlation value for the current active span, when tracing is enabled. */
  spanId?: string
  /** Runs `fn` inside a new span named `operation`, tagged with `safeAttributes` (already sanitized/low-cardinality). */
  withSpan<T>(operation: string, safeAttributes: Record<string, unknown>, fn: () => T | Promise<T>): T | Promise<T>
  /** Records a low-cardinality lifecycle event (e.g. `chat.execute`) with its `outcome` (e.g. `ok`, `error`, `denied`). */
  event(name: string, outcome: string, safeAttributes?: Record<string, unknown>): void
  /** Records a failure classified by a stable `errorCode`; `cause` is sanitized before it ever reaches a client-visible surface. */
  error(name: string, errorCode: string, cause: unknown, safeAttributes?: Record<string, unknown>): void
}
