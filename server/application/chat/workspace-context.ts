import type { ChatOwnershipPort } from './contracts'

/**
 * Resolves workspace path/name for the chat-turn system prompt only after
 * requiring the *session user's own* authorized workspace context — never
 * a naked workspace ID lookup (Plan 031A finding B). A workspace that is
 * missing or belongs to another user degrades the same way (no workspace
 * context in the prompt) instead of leaking another tenant's workspace
 * name/path.
 */
export async function resolveChatWorkspaceContext(userId: string, workspaceId: string | null, ownership: ChatOwnershipPort, resolvePath: (path: string) => Promise<string>) {
  if (!workspaceId) return { path: undefined, name: undefined }
  let workspace
  try {
    workspace = await ownership.findWorkspace(userId, workspaceId)
  } catch {
    return { path: undefined, name: undefined }
  }
  try {
    return { path: await resolvePath(workspace.path), name: workspace.name }
  } catch (err) {
    logger.error('[chat] failed to resolve workspace path for terminal tool', err)
    return { path: undefined, name: undefined }
  }
}

export function buildChatWorkspaceSystemPrompt(path: string | undefined, name: string | undefined) {
  if (!path) return undefined
  return `You are a coding assistant currently working in the workspace "${name}" located at ${path}.`
}
