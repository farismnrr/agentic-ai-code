import { eq, and } from 'drizzle-orm'
import { workspaces } from '../../database/schema'
import * as v from 'valibot'

import fs from 'node:fs/promises'

const updateSchema = v.object({
  name: v.string(),
  path: v.optional(v.string())
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing workspace id')

  const db = useDb()

  const result = v.safeParse(updateSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  const updateData: Record<string, unknown> = {
    name: body.name,
    updatedAt: new Date()
  }

  if (body.path !== undefined) {
    const resolvedPath = await resolveWorkspacePath(body.path)
    try {
      const stat = await fs.stat(resolvedPath)
      if (!stat.isDirectory()) {
        throw createError({ statusCode: 400, statusMessage: 'Path is not a directory' })
      }
    } catch {
      throw createError({ statusCode: 400, statusMessage: 'Invalid or non-existent path' })
    }
    updateData.path = body.path
    updateData.pathConfirmed = true
  }

  const [updated] = await db
    .update(workspaces)
    .set(updateData)
    .where(and(eq(workspaces.id, id), eq(workspaces.userId, session.user.id)))
    .returning()

  if (!updated) {
    throw notFound('Workspace not found')
  }

  return {
    id: updated.id,
    name: updated.name,
    path: updated.path,
    pathConfirmed: updated.pathConfirmed,
    createdAt: updated.createdAt.getTime(),
    updatedAt: updated.updatedAt.getTime()
  }
})
