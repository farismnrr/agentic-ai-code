/** Routes reachable without a session. Everything else is guarded. */
const PUBLIC_ROUTES = ['/', '/login', '/register']

export default defineNuxtRouteMiddleware((to) => {
  // The session lives in localStorage, which doesn't exist on the server, so
  // the server can't know whether anyone is signed in. Guarding there would
  // bounce a signed-in visitor on every hard refresh. App routes are rendered
  // client-side (see `routeRules` in nuxt.config), so there's no flash of
  // guarded content before this runs.
  if (import.meta.server) return

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
