/**
 * Single round trip for everything the sidebar needs: workspaces and
 * lightweight conversation metadata (no message bodies — those come from
 * GET /api/conversations/[id] only when a conversation is opened).
 *
 * This exists specifically so the client never has to orchestrate two
 * separate fetches for one screen — chaining independent composable calls
 * client-side (`loadWorkspaces().then(loadConversations)`, or even two
 * calls placed side by side in a `Promise.allSettled`) repeatedly broke
 * Nuxt's SSR composable context in this codebase (NUXT_E1001, silently
 * swallowed) — see .agents/memories/015-composable-after-await-breaks-ssr-context.md
 * and .agents/memories/018-sidebar-single-fetch.md. The join belongs here,
 * server-side, not as client-side async orchestration.
 */
export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const [ws, convs] = await event.context.application.account.listSidebarData(session.user.id) as [{ id: string, name: string, path: string, pathConfirmed: boolean, createdAt: Date, updatedAt: Date }[], { id: string, title: string, workspaceId: string, modelId: string, reasoningEffort: string | null, enabledToolIds: string[] | null, approvals: unknown, permissionMode: string, mode: string, createdAt: Date, updatedAt: Date }[]]

  return {
    workspaces: ws.map(w => ({
      id: w.id,
      name: w.name,
      path: w.path,
      pathConfirmed: w.pathConfirmed,
      createdAt: w.createdAt.getTime(),
      updatedAt: w.updatedAt.getTime()
    })),
    conversations: convs.map(c => ({
      id: c.id,
      title: c.title,
      workspaceId: c.workspaceId,
      modelId: c.modelId,
      reasoningEffort: c.reasoningEffort,
      enabledToolIds: c.enabledToolIds,
      approvals: c.approvals,
      permissionMode: c.permissionMode,
      mode: c.mode,
      createdAt: c.createdAt.getTime(),
      updatedAt: c.updatedAt.getTime(),
      messages: []
    }))
  }
})
