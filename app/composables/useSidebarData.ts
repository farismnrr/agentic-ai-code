import type { Workspace, Conversation } from '#shared/types/chat'

/**
 * Loads workspaces + conversation metadata for the sidebar in one request
 * (GET /api/sidebar), instead of orchestrating useWorkspaces().loadAll()
 * and useConversations().loadAll() as two separate client-side calls.
 *
 * That two-call shape (`loadWorkspaces(...).then(loadConversations)`, and
 * even placing the two calls side by side in the same `Promise.allSettled`)
 * kept breaking Nuxt's SSR composable context — every extra await/`.then()`
 * boundary between the outer async function's start and the point a
 * composable's body actually runs (e.g. `useRequestFetch()` inside
 * `loadAll()`) loses SSR context, silently, with the failure swallowed by
 * `Promise.allSettled`. See
 * .agents/memories/015-composable-after-await-breaks-ssr-context.md and
 * .agents/memories/018-sidebar-single-fetch.md. One fetch, one assignment,
 * no client-side chaining, removes the whole failure class for this data.
 */
export function useSidebarData() {
  const workspaces = useState<Workspace[]>('workspaces', () => [])
  const conversations = useState<Conversation[]>('conversations', () => [])
  const loaded = useState<boolean>('workspaces-loaded', () => false)
  const activeWorkspaceId = useCookie<string | null>('workspace-id', {
    default: () => null,
    maxAge: 60 * 60 * 24 * 365
  })
  const settings = useSettings()

  const pending = useState<boolean>('sidebar-pending', () => false)
  const error = useState<Error | null>('sidebar-error', () => null)

  /**
   * `settingsPromise` (the in-flight `useSettings().load()` call) is passed
   * in and awaited *inside* this function, alongside the sidebar fetch,
   * rather than awaited by the caller beforehand — same reasoning as
   * useWorkspaces.ts's old loadAll(): an extra sequential await ahead of
   * this call breaks Nuxt's SSR composable-context propagation for the
   * fetch below.
   */
  async function load(settingsPromise?: Promise<unknown>) {
    pending.value = true
    error.value = null
    try {
      const fetch = import.meta.server ? useRequestFetch() : $fetch
      const [data] = await Promise.all([
        fetch<{ workspaces: Workspace[], conversations: Conversation[] }>('/api/sidebar'),
        settingsPromise ?? Promise.resolve()
      ])
      workspaces.value = data.workspaces
      conversations.value = data.conversations

      const lastActiveWorkspaceId = settings.value.lastActiveWorkspaceId
      if (activeWorkspaceId.value && !data.workspaces.some(w => w.id === activeWorkspaceId.value)) {
        activeWorkspaceId.value = null
      } else if (!activeWorkspaceId.value && lastActiveWorkspaceId && data.workspaces.some(w => w.id === lastActiveWorkspaceId)) {
        activeWorkspaceId.value = lastActiveWorkspaceId
      }
    } catch (err) {
      error.value = err as Error
    } finally {
      pending.value = false
      loaded.value = true
    }
  }

  return { load, pending, error }
}
