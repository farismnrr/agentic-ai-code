import { eq } from 'drizzle-orm'
import { workspaces } from '../../database/schema'

export async function resolveChatWorkspaceContext(workspaceId: string | null) {
  if (!workspaceId) return { path: undefined, name: undefined }
  const [workspace] = await useDb().select().from(workspaces).where(eq(workspaces.id, workspaceId)).limit(1)
  if (!workspace) return { path: undefined, name: undefined }
  try {
    return { path: await resolveWorkspacePath(workspace.path), name: workspace.name }
  } catch (err) {
    logger.error('[chat] failed to resolve workspace path for terminal tool', err)
    return { path: undefined, name: undefined }
  }
}

export function buildChatWorkspaceSystemPrompt(path: string | undefined, name: string | undefined) {
  if (!path) return undefined
  return `You are a coding assistant currently working in the workspace "${name}" located at ${path}.`
}
