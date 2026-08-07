import { eq, and } from 'drizzle-orm'
import { workspaces } from '../../database/schema'
import * as v from 'valibot'

const updateSchema = v.object({
  name: v.string()
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing workspace id')

  const db = useDb()

  const result = v.safeParse(updateSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  const [updated] = await db
    .update(workspaces)
    .set({
      name: body.name,
      updatedAt: new Date()
    })
    .where(and(eq(workspaces.id, id), eq(workspaces.userId, session.user.id)))
    .returning()

  if (!updated) {
    throw notFound('Workspace not found')
  }

  return {
    id: updated.id,
    name: updated.name,
    createdAt: updated.createdAt.getTime(),
    updatedAt: updated.updatedAt.getTime()
  }
})
