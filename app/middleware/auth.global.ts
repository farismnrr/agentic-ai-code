/** Routes reachable without a session. Everything else is guarded. */
const PUBLIC_ROUTES = ['/', '/login', '/register']

/**
 * Route guard backed by the nuxt-auth-utils session cookie.
 *
 * The cookie is an httpOnly sealed cookie set by the server, readable on
 * both the server (SSR) and the client (via the nuxt-auth-utils module). This
 * means the guard works correctly on both sides, so the `import.meta.server`
 * early-return that was here before plan 005 is no longer needed.
 *
 * Removing `ssr: false` from /chat/** and /settings/** in nuxt.config.ts
 * means those pages are now server-rendered. The SSR pass runs this guard
 * first, so a signed-in visitor gets their content rendered on the server
 * and an anonymous visitor is redirected to /login — no flash, no client-only
 * rendering workaround.
 */
export default defineNuxtRouteMiddleware((to) => {
  // API handlers own their authentication boundary. Applying the browser
  // navigation guard to `/api/**` turns non-browser callers (for example the
  // relay activity exporter) into HTML login redirects before the server
  // handler can validate its own bearer credential.
  if (to.path.startsWith('/api/')) return

  const { isAuthenticated } = useAuth()
  const isPublic = PUBLIC_ROUTES.includes(to.path)

  if (!isAuthenticated.value && !isPublic) {
    // Carry the destination so signing in returns you where you were headed.
    return navigateTo({ path: '/login', query: { redirect: to.fullPath } })
  }

  if (isAuthenticated.value && (to.path === '/login' || to.path === '/register')) {
    return navigateTo('/chat')
  }
})
