const GENERIC_PROVIDER_ERROR = 'The model provider returned an error. Try again, or switch models.'

export function friendlyChatErrorMessage(error: Error): string {
  const match = error.message.match(/\{.*\}/s)
  if (match) {
    try {
      const parsed = JSON.parse(match[0])
      const nested = parsed?.error?.message
      if (typeof nested === 'string' && nested.length > 0) return nested
    } catch {
      // Not JSON after all — fall through to the raw message below.
    }
  }
  if (!error.message || error.message === '[object Object]') return GENERIC_PROVIDER_ERROR
  return error.message
}
