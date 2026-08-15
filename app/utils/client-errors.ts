/**
 * Client-facing failures must never render transport/provider/runtime error
 * text. Callers provide the UX-specific, bounded copy for the operation.
 */
export function clientErrorMessage(_error: unknown, fallback: string): string {
  return fallback
}
