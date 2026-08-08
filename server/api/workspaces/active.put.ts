import { eq } from 'drizzle-orm'
import { users } from '../../database/schema'
import { z } from 'zod'

const schema = z.object({
  id: z.string().uuid().nullable()
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readValidatedBody(event, schema.parse)

  const db = useDb()
  await db
    .update(users)
    .set({ lastActiveWorkspaceId: body.id })
    .where(eq(users.id, session.user.id))

  return { success: true }
})
