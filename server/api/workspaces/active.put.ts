import { eq } from 'drizzle-orm'
import { users } from '../../database/schema'
import * as v from 'valibot'

const schema = v.object({
  id: v.nullable(v.pipe(v.string(), v.uuid()))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readValidatedBody(event, body => v.parse(schema, body))

  if (body.id !== null) {
    await findUserWorkspace(session.user.id, body.id)
  }

  const db = useDb()
  await db
    .update(users)
    .set({ lastActiveWorkspaceId: body.id })
    .where(eq(users.id, session.user.id))

  return { success: true }
})
