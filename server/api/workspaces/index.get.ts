import { eq, desc } from 'drizzle-orm'
import { workspaces } from '../../database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const db = useDb()

  const userWorkspaces = await db
    .select()
    .from(workspaces)
    .where(eq(workspaces.userId, session.user.id))
    .orderBy(desc(workspaces.updatedAt))

  return userWorkspaces.map(w => ({
    id: w.id,
    name: w.name,
    path: w.path,
    pathConfirmed: w.pathConfirmed,
    createdAt: w.createdAt.getTime(),
    updatedAt: w.updatedAt.getTime()
  }))
})
