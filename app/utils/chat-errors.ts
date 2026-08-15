const GENERIC_PROVIDER_ERROR = 'The model provider returned an error. Try again, or switch models.'
const GENERIC_REQUEST_ERROR = 'The request could not be completed. Please try again.'
const GENERIC_RELAY_ERROR = 'The local relay could not complete the request. Check that it is running and try again.'
const REQUEST_CANCELLED = 'Request cancelled.'

function isCancellation(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false
  const candidate = error as { name?: unknown, code?: unknown }
  return candidate.name === 'AbortError' || candidate.code === 'ABORT_ERR'
}

/** Client-facing errors are category-only; raw transport/provider text is private. */
export function friendlyChatErrorMessage(error: unknown): string {
  return isCancellation(error) ? REQUEST_CANCELLED : GENERIC_PROVIDER_ERROR
}

export function friendlyRequestErrorMessage(error: unknown): string {
  return isCancellation(error) ? REQUEST_CANCELLED : GENERIC_REQUEST_ERROR
}

export function friendlyRelayErrorMessage(error: unknown): string {
  return isCancellation(error) ? REQUEST_CANCELLED : GENERIC_RELAY_ERROR
}
