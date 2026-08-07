/**
 * GET /api/me
 *
 * Returns the current session user. The client uses this to bootstrap the
 * session on a hard refresh without a separate API call — `useUserSession()`
 * reads the cookie directly in most cases, but having an explicit endpoint
 * makes debugging easier and gives a clean 401 path for unauthenticated requests.
 */
export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return session.user
})
