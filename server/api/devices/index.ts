import { eq } from 'drizzle-orm'
import * as v from 'valibot'
import { userDevices } from '#server/database/schema'

const registerDeviceSchema = v.object({
  name: v.pipe(v.string(), v.minLength(1, 'Device name is required')),
  fingerprint: v.pipe(v.string(), v.minLength(1, 'Fingerprint is required'))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const userId = session.user.id
  const method = event.method

  const db = useDb()

  if (method === 'GET') {
    const devices = await db.select().from(userDevices).where(eq(userDevices.userId, userId))
    return devices
  }

  if (method === 'POST') {
    const result = v.safeParse(registerDeviceSchema, await readBody(event))
    if (!result.success) throw unprocessable(result.issues)
    const { name, fingerprint } = result.output

    const [device] = await db.insert(userDevices).values({
      userId,
      name,
      fingerprint,
      pairedAt: new Date(),
      lastSeenAt: new Date()
    }).returning()

    return device
  }

  throw badRequest(`Unsupported method: ${method}`)
})
