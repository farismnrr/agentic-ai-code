import { eq, and } from 'drizzle-orm'
import { userDevices } from '#server/database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const userId = session.user.id
  const deviceId = getRouterParam(event, 'id')

  if (!deviceId) {
    throw createError({ statusCode: 400, statusMessage: 'Device ID is required' })
  }

  const db = useDatabase()

  const [device] = await db.update(userDevices)
    .set({ revokedAt: new Date() })
    .where(and(eq(userDevices.id, deviceId), eq(userDevices.userId, userId)))
    .returning()

  if (!device) {
    throw createError({ statusCode: 404, statusMessage: 'Device not found' })
  }

  return { success: true, device }
})
