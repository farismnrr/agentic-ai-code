import type { Workspace } from '#shared/types/chat'

export function useWorkspaces() {
  const workspaces = useState<Workspace[]>('workspaces', () => [])
  // Persist the active workspace across reloads
  const activeWorkspaceId = useCookie<string | null>('workspace-id', { default: () => null })

  const sorted = computed(() =>
    [...workspaces.value].sort((a, b) => b.updatedAt - a.updatedAt)
  )

  function get(id: string): Workspace | undefined {
    return workspaces.value.find(w => w.id === id)
  }

  async function loadAll() {
    const data = await $fetch<Workspace[]>('/api/workspaces')
    workspaces.value = data

    // If we have workspaces but no active one, or the active one doesn't exist anymore, default to the first one
    if (data.length > 0) {
      if (!activeWorkspaceId.value || !data.some(w => w.id === activeWorkspaceId.value)) {
        activeWorkspaceId.value = data[0].id
      }
    } else {
      activeWorkspaceId.value = null
    }
  }

  async function create(name: string): Promise<Workspace> {
    const data = await $fetch<Workspace>('/api/workspaces', {
      method: 'POST',
      body: { name }
    })
    workspaces.value = [data, ...workspaces.value]
    return data
  }

  function updateLocally(id: string, patch: Partial<Workspace>) {
    workspaces.value = workspaces.value.map(w =>
      w.id === id ? { ...w, ...patch, updatedAt: patch.updatedAt || Date.now() } : w
    )
  }

  async function update(id: string, name: string) {
    updateLocally(id, { name })
    const data = await $fetch<Workspace>(`/api/workspaces/${id}`, {
      method: 'PUT',
      body: { name }
    })
    updateLocally(id, data)
  }

  async function remove(id: string) {
    const original = [...workspaces.value]
    workspaces.value = workspaces.value.filter(w => w.id !== id)

    // Switch active if we just deleted it
    if (activeWorkspaceId.value === id) {
      const remaining = sorted.value
      activeWorkspaceId.value = remaining.length > 0 ? remaining[0].id : null
    }

    try {
      await $fetch(`/api/workspaces/${id}`, { method: 'DELETE' })
    } catch (e) {
      // Revert on failure
      workspaces.value = original
      throw e
    }
  }

  return { workspaces, sorted, activeWorkspaceId, get, loadAll, create, update, remove }
}
