import { eq, desc, and } from 'drizzle-orm'
import { workspaces } from '../database/schema'
import fs from 'node:fs/promises'
import { resolveWorkspacePath } from './fs-browse'

export async function listWorkspaces(userId: string) {
  const db = useDb()
  const result = await db
    .select()
    .from(workspaces)
    .where(eq(workspaces.userId, userId))
    .orderBy(desc(workspaces.updatedAt))

  return result.map(w => ({
    id: w.id,
    name: w.name,
    path: w.path,
    pathConfirmed: w.pathConfirmed,
    createdAt: w.createdAt.getTime(),
    updatedAt: w.updatedAt.getTime()
  }))
}

export async function createWorkspace(userId: string, name: string, path: string) {
  const resolvedPath = await resolveWorkspacePath(path)
  try {
    const stat = await fs.stat(resolvedPath)
    if (!stat.isDirectory()) {
      throw createError({ statusCode: 400, statusMessage: 'Path is not a directory' })
    }
  } catch {
    throw createError({ statusCode: 400, statusMessage: 'Invalid or non-existent path' })
  }

  const db = useDb()
  const [workspace] = await db
    .insert(workspaces)
    .values({
      userId,
      name,
      path,
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
}

export async function updateWorkspace(userId: string, id: string, name: string, path?: string) {
  const updateData: Record<string, unknown> = {
    name,
    updatedAt: new Date()
  }

  if (path !== undefined) {
    const resolvedPath = await resolveWorkspacePath(path)
    try {
      const stat = await fs.stat(resolvedPath)
      if (!stat.isDirectory()) {
        throw createError({ statusCode: 400, statusMessage: 'Path is not a directory' })
      }
    } catch {
      throw createError({ statusCode: 400, statusMessage: 'Invalid or non-existent path' })
    }
    updateData.path = path
    updateData.pathConfirmed = true
  }

  const db = useDb()
  const [workspace] = await db
    .update(workspaces)
    .set(updateData)
    .where(and(eq(workspaces.id, id), eq(workspaces.userId, userId)))
    .returning()

  if (!workspace) {
    throw createError({ statusCode: 404, statusMessage: 'Workspace not found' })
  }

  return {
    id: workspace.id,
    name: workspace.name,
    path: workspace.path,
    pathConfirmed: workspace.pathConfirmed,
    createdAt: workspace.createdAt.getTime(),
    updatedAt: workspace.updatedAt.getTime()
  }
}

export async function deleteWorkspace(userId: string, id: string) {
  const db = useDb()

  // Guard against deleting the last workspace
  const userWorkspaces = await db
    .select({ id: workspaces.id })
    .from(workspaces)
    .where(eq(workspaces.userId, userId))

  if (userWorkspaces.length <= 1) {
    throw createError({ statusCode: 400, statusMessage: 'Cannot delete the last workspace' })
  }

  const [deleted] = await db
    .delete(workspaces)
    .where(and(eq(workspaces.id, id), eq(workspaces.userId, userId)))
    .returning()

  if (!deleted) {
    throw createError({ statusCode: 404, statusMessage: 'Workspace not found' })
  }

  return { ok: true }
}
