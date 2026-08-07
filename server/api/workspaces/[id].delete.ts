import { eq, and } from 'drizzle-orm'
import { workspaces } from '../../database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing workspace id')

  const db = useDb()

  // Guard against deleting the last workspace
  const userWorkspaces = await db
    .select({ id: workspaces.id })
    .from(workspaces)
    .where(eq(workspaces.userId, session.user.id))

  if (userWorkspaces.length <= 1) {
    throw badRequest('Cannot delete the last workspace')
  }

  const [deleted] = await db
    .delete(workspaces)
    .where(and(eq(workspaces.id, id), eq(workspaces.userId, session.user.id)))
    .returning()

  if (!deleted) {
    throw notFound('Workspace not found')
  }

  return { ok: true }
})
