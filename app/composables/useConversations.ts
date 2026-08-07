import type { Conversation, UIMessage } from '#shared/types/chat'

/**
 * In-memory conversation store.
 *
 * Backed by `useState` rather than a module-scope `ref`: on the server a
 * module-scope ref is shared across every request, so one visitor's
 * conversations would leak into another's. State resets on reload by design —
 * persistence is out of scope for this iteration.
 */
export function useConversations() {
  const conversations = useState<Conversation[]>('conversations', () => [])
  const settings = useSettings()

  /** Newest first, which is the order the sidebar renders. */
  const sorted = computed(() =>
    [...conversations.value].sort((a, b) => b.updatedAt - a.updatedAt)
  )

  function get(id: string): Conversation | undefined {
    return conversations.value.find(c => c.id === id)
  }

  async function loadAll() {
    const { activeWorkspaceId } = useWorkspaces()
    if (!activeWorkspaceId.value) {
      conversations.value = []
      return
    }
    const data = await $fetch<Conversation[]>('/api/conversations', {
      query: { workspaceId: activeWorkspaceId.value }
    })
    conversations.value = data
  }

  async function loadOne(id: string) {
    try {
      const data = await $fetch<Conversation>(`/api/conversations/${id}`)
      updateLocally(id, data)
      return data
    } catch {
      return null
    }
  }

  async function create(overrides: Partial<Conversation> = {}): Promise<Conversation> {
    const { activeWorkspaceId } = useWorkspaces()
    if (!activeWorkspaceId.value) throw new Error('No active workspace')
    const data = await $fetch<Conversation>('/api/conversations', {
      method: 'POST',
      body: {
        workspaceId: activeWorkspaceId.value,
        title: overrides.title || 'New chat',
        modelId: overrides.modelId || settings.value.defaultModelId
      }
    })
    conversations.value = [data, ...conversations.value]
    return data
  }

  function updateLocally(id: string, patch: Partial<Conversation>) {
    conversations.value = conversations.value.map(c =>
      c.id === id ? { ...c, ...patch, updatedAt: patch.updatedAt || Date.now() } : c
    )
  }

  async function update(id: string, patch: Partial<Conversation>) {
    // Optimistic update
    updateLocally(id, patch)

    // Pick fields that are safe to update directly
    const apiPatch: Pick<Partial<Conversation>, 'title' | 'enabledToolIds' | 'approvals'> = {}
    if (patch.title !== undefined) apiPatch.title = patch.title
    if (patch.enabledToolIds !== undefined) apiPatch.enabledToolIds = patch.enabledToolIds
    if (patch.approvals !== undefined) apiPatch.approvals = patch.approvals

    if (Object.keys(apiPatch).length > 0) {
      const data = await $fetch<Conversation>(`/api/conversations/${id}`, {
        method: 'PUT',
        body: apiPatch
      })
      updateLocally(id, data)
    }
  }

  /** Restores seed data — the way out of a wedged demo, since nothing persists. */
  function reset() {
    // not used much now
  }

  async function remove(id: string) {
    conversations.value = conversations.value.filter(c => c.id !== id)
    await $fetch(`/api/conversations/${id}`, { method: 'DELETE' })
  }

  function setMessages(id: string, messages: UIMessage[]) {
    // We only update messages locally in useConversations because chat persistence
    // to DB happens in the chat stream itself / backend.
    updateLocally(id, { messages })
  }

  /**
   * Derive a title from the first user message. ChatGPT does this server-side
   * with a model call; without a backend the first line is a good stand-in.
   */
  function titleFrom(text: string): string {
    const firstLine = text.trim().split('\n')[0] ?? ''
    const trimmed = firstLine.slice(0, 48).trim()
    if (!trimmed) return 'New chat'
    return firstLine.length > 48 ? `${trimmed}…` : trimmed
  }

  return { conversations, sorted, get, loadAll, loadOne, create, update, updateLocally, remove, reset, setMessages, titleFrom }
}
