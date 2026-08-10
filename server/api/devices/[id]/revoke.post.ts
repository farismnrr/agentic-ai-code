import { eq, and } from 'drizzle-orm'
import * as v from 'valibot'
import { userDevices } from '#server/database/schema'

const paramsSchema = v.object({
  id: v.pipe(v.string(), v.uuid('Invalid device ID format'))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const userId = session.user.id

  const result = v.safeParse(paramsSchema, { id: getRouterParam(event, 'id') })
  if (!result.success) throw unprocessable(result.issues)

  const db = useDb()

  const [device] = await db.update(userDevices)
    .set({ revokedAt: new Date() })
    .where(and(eq(userDevices.id, result.output.id), eq(userDevices.userId, userId)))
    .returning()

  if (!device) {
    throw notFound('Device not found')
  }

  return { success: true, device }
})
