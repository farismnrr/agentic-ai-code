/**
 * POST /api/auth/logout
 *
 * Clears the session cookie. No body required — the identity to sign out
 * is already known from the cookie.
 */
export default defineEventHandler(async (event) => {
  await clearUserSession(event)
  return { ok: true }
})
