/**
 * Thin wrapper over nuxt-auth-utils' `useUserSession()`.
 *
 * Call-sites (login.vue, register.vue, default layout) stay unchanged — they
 * still see `user`, `isAuthenticated`, `login`, `register`, and `logout`.
 *
 * The heavy lifting moved to the server:
 * - Session is a sealed httpOnly cookie (nuxt-auth-utils).
 * - Credentials are validated and hashed server-side (scrypt).
 * - This composable only communicates with those server routes.
 *
 * The `restore()` helper and `localStorage` paths are removed entirely —
 * the cookie is readable on both server and client, so there's nothing to
 * restore manually.
 */

export interface SessionUser {
  id: string
  name: string
  email: string
  /** ISO string when verified, null when unverified. */
  emailVerifiedAt: string | null
}

export function useAuth() {
  const { user: sessionUser, fetch: fetchSession, clear } = useUserSession()

  /**
   * The session user, typed. nuxt-auth-utils stores it as `Record<string, unknown>`,
   * so we cast here — the server populates exactly these fields.
   */
  const user = computed(() =>
    sessionUser.value ? (sessionUser.value as unknown as SessionUser) : null
  )

  const isAuthenticated = computed(() => user.value !== null)

  /**
   * Send credentials to the server and receive a session cookie in return.
   * On success the cookie is set automatically; `useUserSession()` will
   * reflect the new user without a reload (it's reactive).
   */
  async function login(email: string, password: string) {
    await $fetch('/api/auth/login', {
      method: 'POST',
      body: { email, password }
    })
    // nuxt-auth-utils updates the session reactive state after the
    // Set-Cookie header arrives. fetch() re-reads the session endpoint
    // and refreshes the reactive user state.
    await fetchSession()
  }

  /**
   * Creates an account and establishes a session in one step.
   * All four parameters are validated server-side as well.
   */
  async function register(name: string, email: string, password: string, confirm: string) {
    await $fetch('/api/auth/register', {
      method: 'POST',
      body: { name, email, password, confirm }
    })
    await fetchSession()
  }

  /** Deletes the session cookie via the server route and clears local state. */
  async function logout() {
    await $fetch('/api/auth/logout', { method: 'POST' })
    await clear()
  }

  return { user, isAuthenticated, login, register, logout }
}
