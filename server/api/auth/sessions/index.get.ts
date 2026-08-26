import { browserSessionFrom } from '../../../application/auth-session'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const current = browserSessionFrom(await getUserSession(event))
  const sessions = await event.context.application.account.listAuthSessions(session.user.id) as Array<{ id: string, createdAt: Date, lastSeenAt: Date, revokedAt?: Date | null }>
  return sessions.map(item => ({
    id: item.id,
    createdAt: item.createdAt,
    lastSeenAt: item.lastSeenAt,
    current: item.id === current?.id
  }))
})
