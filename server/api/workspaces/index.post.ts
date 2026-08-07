import { workspaces } from '../../database/schema'
import * as v from 'valibot'

const createSchema = v.object({
  name: v.string()
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const db = useDb()

  const result = v.safeParse(createSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  const [workspace] = await db
    .insert(workspaces)
    .values({
      userId: session.user.id,
      name: body.name
    })
    .returning()

  if (!workspace) {
    throw internal('Failed to create workspace')
  }

  return {
    id: workspace.id,
    name: workspace.name,
    createdAt: workspace.createdAt.getTime(),
    updatedAt: workspace.updatedAt.getTime()
  }
})
