import { getSettings } from '../application/features'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return getSettings(session.user.id, session.user.name, session.user.email)
})
