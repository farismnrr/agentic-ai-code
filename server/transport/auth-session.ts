import type { H3Event } from 'h3'
import { hashSessionSecret, issueBrowserAuthSession, type AuthSessionUser } from '../application/auth-session'

/** Transport/framework adapter for creating a browser session. */
export async function establishAuthSession(event: H3Event, user: AuthSessionUser) {
  const authSession = issueBrowserAuthSession()
  const application = event.context.application
  await application.account.createAuthSession({
    id: authSession.id,
    userId: user.id,
    secretHash: hashSessionSecret(authSession.secret)
  })
  await setUserSession(event, { user, secure: { authSession } })
}
