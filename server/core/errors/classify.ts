// Plan 035 P1 remediation (round 4): bounded, static classification of a
// raw/untrusted exception's likely cause — mirrors the Rust
// `classify_reqwest_error()` pattern (packages/rust-tools/infrastructure/src/
// observability.rs): map the error's SHAPE (a driver/runtime error code, a
// well-known constructor name) to a fixed vocabulary of safe labels, never
// echo the error's own free-text `.message`. That free text can carry
// request-derived/PII data (e.g. a Postgres unique-constraint violation
// embedding a submitted email) that no credential/path regex would catch.
//
// Deliberately conservative: anything that doesn't match a known safe shape
// classifies as 'unclassified' rather than guessing from message content —
// fail-closed per the frozen contract, not a denylist of "bad" patterns.
export type SafeCauseClassification = string

const CODE_PATTERN = /^[A-Za-z0-9_-]{1,32}$/

export function classifyRawCause(cause: unknown): SafeCauseClassification {
  if (cause === undefined || cause === null) return 'unknown'
  if (!(cause instanceof Error)) return 'unclassified'

  // Driver/runtime error codes are bounded, static identifiers by
  // construction (Postgres SQLSTATE like '23505', Node's 'ECONNREFUSED',
  // etc.) — safe to surface directly, unlike free-text `.message`.
  const code = (cause as { code?: unknown }).code
  if (typeof code === 'string' && CODE_PATTERN.test(code)) return code
  if (typeof code === 'number' && Number.isFinite(code)) return `code_${code}`

  const name = cause.name || cause.constructor?.name
  if (name === 'AbortError') return 'aborted'
  if (name === 'TimeoutError') return 'timeout'

  return 'unclassified'
}
