export default defineEventHandler((event) => {
  const path = getRequestURL(event).pathname
  setResponseHeader(event, 'X-Content-Type-Options', 'nosniff')
  setResponseHeader(event, 'X-Frame-Options', 'DENY')
  setResponseHeader(event, 'Referrer-Policy', 'strict-origin-when-cross-origin')
  setResponseHeader(event, 'Permissions-Policy', 'camera=(), microphone=(), geolocation=(), payment=()')
  setResponseHeader(event, 'Cross-Origin-Opener-Policy', 'same-origin')
  setResponseHeader(event, 'Cross-Origin-Resource-Policy', 'same-origin')
  setResponseHeader(event, 'X-Robots-Tag', 'noindex, nofollow')
  if (path.startsWith('/api/auth/') || path.startsWith('/api/security/')) {
    setResponseHeader(event, 'Cache-Control', 'no-store')
    setResponseHeader(event, 'Content-Security-Policy', `default-src 'none'; frame-ancestors 'none'; base-uri 'none'`)
  } else if (path.startsWith('/api/')) {
    setResponseHeader(event, 'Cache-Control', 'no-store')
  } else {
    setResponseHeader(event, 'Content-Security-Policy', `frame-ancestors 'none'; base-uri 'self'; object-src 'none'`)
  }
})
