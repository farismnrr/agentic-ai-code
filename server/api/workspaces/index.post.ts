import { workspaces } from '../../database/schema'
import * as v from 'valibot'

import fs from 'node:fs/promises'

const createSchema = v.object({
  name: v.string(),
  path: v.string()
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const db = useDb()

  const result = v.safeParse(createSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  // Validate path resolves within root and exists as a directory
  const resolvedPath = resolveWorkspacePath(body.path)
  try {
    const stat = await fs.stat(resolvedPath)
    if (!stat.isDirectory()) {
      throw createError({ statusCode: 400, statusMessage: 'Path is not a directory' })
    }
  } catch {
    throw createError({ statusCode: 400, statusMessage: 'Invalid or non-existent path' })
  }

  const [workspace] = await db
    .insert(workspaces)
    .values({
      userId: session.user.id,
      name: body.name,
      path: body.path,
      pathConfirmed: true
    })
    .returning()

  if (!workspace) {
    throw internal('Failed to create workspace')
  }

  return {
    id: workspace.id,
    name: workspace.name,
    path: workspace.path,
    pathConfirmed: workspace.pathConfirmed,
    createdAt: workspace.createdAt.getTime(),
    updatedAt: workspace.updatedAt.getTime()
  }
})
