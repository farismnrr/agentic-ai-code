// Plan 035 P1 remediation (round 4): marker for a diagnostic message a
// developer deliberately wrote as safe (a static/templated string with no
// interpolated raw exception text), as opposed to a raw caught exception's
// own `.message`. `redactSecrets()`/Rust `redact_secrets()` only mask
// credential-shaped substrings and filesystem paths — they are NOT a
// general PII/data-classification boundary, so raw `Error.message` (which
// can carry request-derived values like a DB unique-constraint violation
// embedding a user's email) must never reach private telemetry by default.
// Wrapping a message in `SafeDiagnosticError` is how a call site opts a
// diagnostic string INTO being logged verbatim — because a human decided
// it is safe, not because it happened to come from `.message`.
export class SafeDiagnosticError extends Error {
  readonly isSafeDiagnostic = true as const

  constructor(message: string) {
    super(message)
    this.name = 'SafeDiagnosticError'
  }
}

export const safeDiagnostic = (message: string) => new SafeDiagnosticError(message)

export function isSafeDiagnostic(err: unknown): err is SafeDiagnosticError {
  return err instanceof SafeDiagnosticError
}
