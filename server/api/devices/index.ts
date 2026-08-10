import { eq } from 'drizzle-orm'
import { userDevices } from '#server/database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const userId = session.user.id
  const method = event.method

  const db = useDatabase()

  if (method === 'GET') {
    const devices = await db.select().from(userDevices).where(eq(userDevices.userId, userId))
    return devices
  }

  if (method === 'POST') {
    const body = await readBody(event)
    const { name, fingerprint } = body || {}

    if (!name || !fingerprint) {
      throw createError({ statusCode: 400, statusMessage: 'Name and fingerprint are required' })
    }

    const [device] = await db.insert(userDevices).values({
      userId,
      name,
      fingerprint,
      pairedAt: new Date(),
      lastSeenAt: new Date()
    }).returning()

    return device
  }

  throw createError({ statusCode: 405, statusMessage: 'Method not allowed' })
})
