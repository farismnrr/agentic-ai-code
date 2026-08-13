import type { Workspace } from '#shared/types/chat'
import { removeById, replaceById } from '../utils/collection'

export function useWorkspaces() {
  const workspaces = useState<Workspace[]>('workspaces', () => [])
  const loaded = useState<boolean>('workspaces-loaded', () => false)
  // Persist the active workspace across reloads
  const activeWorkspaceId = useCookie<string | null>('workspace-id', {
    default: () => null,
    maxAge: 60 * 60 * 24 * 365
  })
  // Grabbed synchronously here, not inside loadAll() after its await —
  // calling a composable (useSettings() -> useState() -> useNuxtApp())
  // after an await loses Nuxt's SSR request context (NUXT_E1001, with a
  // stack trace that otherwise gives no hint anything failed). Reading
  // `.value` later, after the await, is fine — that's a plain property
  // read, not a composable call.
  const settings = useSettings()

  const sorted = computed(() =>
    [...workspaces.value].sort((a, b) => b.updatedAt - a.updatedAt)
  )

  function get(id: string): Workspace | undefined {
    return workspaces.value.find(w => w.id === id)
  }

  function setActive(id: string | null) {
    activeWorkspaceId.value = id

    // Fire and forget, but log errors
    if (import.meta.client) {
      $fetch('/api/workspaces/active', {
        method: 'PUT',
        body: { id }
      }).catch(e => console.error('Failed to persist active workspace:', e))
    }
  }

  /**
   * `settingsPromise` (the in-flight `useSettings().load()` call) is
   * awaited *inside* this function, alongside the workspaces fetch,
   * rather than sequentially before it — an extra `await` inserted ahead
   * of the caller's `Promise.allSettled` broke Nuxt's SSR
   * composable-context propagation for this and the other calls in that
   * same batch (surfaced as NUXT_E1001 with no other symptom). Awaiting
   * it here, inside the existing parallel fetch, avoids that.
   *
   * The restore itself has to happen *inside* this function, before
   * `loaded` flips true in the `finally` below — pages gate their "show
   * the workspace picker" decision on `loaded`, so if it went true first,
   * the picker would already be locked into the render before a
   * caller-side restore could catch up.
   */
  async function loadAll(settingsPromise?: Promise<unknown>) {
    try {
      const fetch = import.meta.server ? useRequestFetch() : $fetch
      const [data] = await Promise.all([
        fetch<Workspace[]>('/api/workspaces'),
        settingsPromise ?? Promise.resolve()
      ])
      workspaces.value = data

      const lastActiveWorkspaceId = settings.value.lastActiveWorkspaceId

      if (activeWorkspaceId.value && !data.some(w => w.id === activeWorkspaceId.value)) {
        // Clear active workspace if it no longer exists
        setActive(null)
      } else if (!activeWorkspaceId.value && lastActiveWorkspaceId && data.some(w => w.id === lastActiveWorkspaceId)) {
        // No cookie (new browser/device, or it expired) but the server
        // remembers the last pick and it's still a real workspace — restore it.
        setActive(lastActiveWorkspaceId)
      }
    } finally {
      loaded.value = true
    }
  }

  async function create(name: string, path: string): Promise<Workspace> {
    const data = await $fetch<Workspace>('/api/workspaces', {
      method: 'POST',
      body: { name, path }
    })
    workspaces.value = [data, ...workspaces.value]
    return data
  }

  function updateLocally(id: string, patch: Partial<Workspace>) {
    const current = workspaces.value.find(w => w.id === id)
    if (current) workspaces.value = replaceById(workspaces.value, id, { ...current, ...patch, updatedAt: patch.updatedAt || Date.now() })
  }

  async function update(id: string, updates: Partial<Workspace>) {
    updateLocally(id, updates)
    const data = await $fetch<Workspace>(`/api/workspaces/${id}`, {
      method: 'PUT',
      body: updates
    })
    updateLocally(id, data)
  }

  async function remove(id: string) {
    const original = [...workspaces.value]
    workspaces.value = removeById(workspaces.value, id)

    // Switch active if we just deleted it
    if (activeWorkspaceId.value === id) {
      const remaining = sorted.value
      setActive(remaining.length > 0 ? remaining[0]!.id : null)
    }

    try {
      await $fetch(`/api/workspaces/${id}` as string, { method: 'DELETE' })
    } catch (e) {
      // Revert on failure
      workspaces.value = original
      throw e
    }
  }

  return { workspaces, sorted, activeWorkspaceId, loaded, get, loadAll, create, update, remove, setActive }
}
