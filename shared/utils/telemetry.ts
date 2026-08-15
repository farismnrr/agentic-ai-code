// Plan 035 Phase 5 — telemetry event vocabulary shared between the frontend
// producer (`app/composables/useTelemetry.ts`) and the server ingestion
// endpoint (`server/api/telemetry.post.ts`), so the two never drift apart.
// Kept as a plain array/set, not a class/factory — small closed vocabulary
// per the plan's anti-overengineering rule. Must be kept byte-for-byte in
// sync with the `TelemetryEventName` union in `useTelemetry.ts`.
export const TELEMETRY_EVENT_NAMES = [
  'page.error',
  'page.unhandled_rejection',
  'api.request.success',
  'api.request.error',
  'chat.stream.start',
  'chat.stream.persist'
] as const

export type TelemetryEventName = typeof TELEMETRY_EVENT_NAMES[number]

export function isTelemetryEventName(value: unknown): value is TelemetryEventName {
  return typeof value === 'string' && (TELEMETRY_EVENT_NAMES as readonly string[]).includes(value)
}
