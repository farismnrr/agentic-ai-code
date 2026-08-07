/**
 * Restores the session from localStorage before the app renders.
 *
 * `enforce: 'pre'` matters: the global auth middleware reads `isAuthenticated`,
 * and on a hard refresh into a guarded route the middleware would otherwise run
 * against an empty session and redirect a logged-in user to /login.
 */
export default defineNuxtPlugin({
  name: 'auth-restore',
  enforce: 'pre',
  setup() {
    useAuth().restore()
  }
})
