import { readFileSync } from 'node:fs'

function read(path: string) {
  return readFileSync(new URL(`../../${path}`, import.meta.url), 'utf8')
}

function requireText(source: string, needle: string, message: string) {
  if (!source.includes(needle)) throw new Error(message)
}

const forgot = read('server/api/auth/forgot.post.ts')
const reset = read('server/api/auth/reset.post.ts')
const login = read('server/api/auth/login.post.ts')
const register = read('server/api/auth/register.post.ts')
const forgotPage = read('app/pages/forgot-password.vue')
const resetPage = read('app/pages/reset-password.vue')
const authDb = read('server/infrastructure/database/auth.ts')
const middleware = read('server/middleware/api-auth.ts')
const schema = read('server/database/schema.ts')
const migration = read('server/database/migrations/0017_chunky_betty_brant.sql')
const github = read('server/routes/auth/github.get.ts')
const google = read('server/routes/auth/google.get.ts')

requireText(forgot, 'return { ok: true }', 'forgot-password must use a generic success response')
requireText(forgot, '30 * 60 * 1000', 'password-reset tokens must keep a short 30-minute TTL')
requireText(forgot, 'generateToken()', 'password-reset tokens must come from the cryptographic token generator')
requireText(forgot, 'tokenHash,', 'password-reset persistence must store the token hash, not the bearer token')
requireText(forgot, '/reset-password#token=${token}', 'new reset links must keep bearer tokens out of HTTP request URLs')
requireText(forgotPage, 'If an account exists for', 'forgot-password UI must not confirm account existence')
requireText(resetPage, 'window.location.hash.slice(1)', 'reset page must read new reset tokens from the URL fragment')
requireText(resetPage, `window.history.replaceState(null, '', window.location.pathname)`, 'reset page must scrub bearer credentials after reading them')
requireText(reset, 'hashToken(body.token)', 'reset endpoint must hash the submitted bearer token before lookup')
requireText(authDb, `eq(verificationTokens.type, 'password_reset')`, 'reset consumption must scope tokens by purpose')
requireText(authDb, 'isNull(verificationTokens.consumedAt)', 'reset consumption must reject replayed tokens atomically')
requireText(authDb, 'gt(verificationTokens.expiresAt, now)', 'reset consumption must reject expired tokens before password mutation')
requireText(authDb, '.returning()', 'reset consumption must acquire the token through the guarded mutation')
requireText(authDb, 'authVersion: sql`${users.authVersion} + 1`', 'password reset must increment the session revocation generation')
requireText(authDb, 'tx.delete(verificationTokens)', 'issuing a new reset token must revoke prior reset tokens for that user')
requireText(authDb, '.set({ consumedAt: now })', 'successful reset must invalidate any remaining reset links')
requireText(reset, 'await clearUserSession(event)', 'successful reset must clear the current browser session immediately')
requireText(middleware, '(sessionUser.authVersion ?? 0) !== current.authVersion', 'session middleware must reject stale sealed-cookie generations')
requireText(middleware, 'await clearUserSession(event)', 'stale sessions must be cleared server-side')
requireText(schema, `authVersion: integer('auth_version').notNull().default(0)`, 'users must persist an auth session generation')
requireText(migration, 'ADD COLUMN "auth_version" integer DEFAULT 0 NOT NULL', 'auth-version migration must exist')
if (migration.includes('permission_mode')) throw new Error('account-recovery migration must not replay the existing permission_mode migration')
requireText(login, 'authVersion: user.authVersion', 'password login sessions must carry the auth generation')
requireText(register, 'authVersion: created.authVersion', 'registration sessions must carry the auth generation')
requireText(middleware, 'authVersion: user.authVersion', 'API-key seeded sessions must carry the auth generation')
requireText(github, 'authVersion: user.authVersion', 'GitHub OAuth sessions must carry the auth generation')
requireText(google, 'authVersion: user.authVersion', 'Google OAuth sessions must carry the auth generation')

console.log('account-recovery-security: OK')
