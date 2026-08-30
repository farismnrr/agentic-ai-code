const SAFE_METHODS = new Set(['GET', 'HEAD', 'OPTIONS'])

export interface RequestSecurityInput {
  method: string
  path: string
  origin?: string | null
  siteUrl?: string | null
  authorization?: string | null
  secFetchSite?: string | null
  contentType?: string | null
  contentLength?: string | null
  transferEncoding?: string | null
}

export function isUnsafeApiMutation(input: Pick<RequestSecurityInput, 'method' | 'path'>) {
  return !SAFE_METHODS.has(input.method.toUpperCase()) && input.path.startsWith('/api/')
}

export function isBearerApiRequest(authorization?: string | null) {
  return Boolean(authorization?.startsWith('Bearer aic_live_'))
}

export function allowsMutationOrigin(input: RequestSecurityInput) {
  if (!isUnsafeApiMutation(input)) return true

  if (input.origin) {
    if (!input.siteUrl) return false
    try {
      return input.origin === new URL(input.siteUrl).origin
    } catch {
      return false
    }
  }

  // Non-browser API clients authenticate with an explicit bearer credential and
  // are not vulnerable to ambient-cookie CSRF. Cookie-authenticated browser
  // mutations must prove same-origin when Origin is unavailable.
  if (isBearerApiRequest(input.authorization)) return true
  return input.secFetchSite === 'same-origin'
}

export function allowsMutationContentType(input: RequestSecurityInput) {
  if (!isUnsafeApiMutation(input)) return true

  const contentType = input.contentType?.trim().toLowerCase()
  if (contentType) return contentType.startsWith('application/json')

  // Bodyless mutation endpoints (for example logout) may omit Content-Type.
  // A request that advertises a body must explicitly declare JSON.
  const length = Number(input.contentLength ?? '0')
  const hasLengthBody = Number.isFinite(length) && length > 0
  const hasStreamedBody = Boolean(input.transferEncoding?.trim())
  return !hasLengthBody && !hasStreamedBody
}

export function securityHeadersForPath(path: string) {
  const headers: Record<string, string> = {
    'X-Content-Type-Options': 'nosniff',
    'X-Frame-Options': 'DENY',
    'Referrer-Policy': 'strict-origin-when-cross-origin',
    'Permissions-Policy': 'camera=(), microphone=(), geolocation=(), payment=()',
    'Cross-Origin-Opener-Policy': 'same-origin',
    'Cross-Origin-Resource-Policy': 'same-origin',
    'X-Robots-Tag': 'noindex, nofollow'
  }

  if (path.startsWith('/api/auth/') || path.startsWith('/api/security/')) {
    headers['Cache-Control'] = 'no-store'
    headers['Content-Security-Policy'] = `default-src 'none'; frame-ancestors 'none'; base-uri 'none'`
  } else if (path.startsWith('/api/')) {
    headers['Cache-Control'] = 'no-store'
  } else {
    headers['Content-Security-Policy'] = `frame-ancestors 'none'; base-uri 'self'; object-src 'none'`
  }

  return headers
}
