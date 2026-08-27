import type { Conversation, UIMessage } from '#shared/types/chat'
import { removeById, replaceById } from '../utils/collection'

/**
 * In-memory conversation store.
 *
 * Backed by `useState` rather than a module-scope `ref`: on the server a
 * module-scope ref is shared across every request, so one visitor's
 * conversations would leak into another's.
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
    const fetch = import.meta.server ? useRequestFetch() : $fetch
    const data = await fetch<Conversation[]>('/api/conversations')
    conversations.value = data
  }

  async function loadOne(id: string) {
    try {
      const fetch = import.meta.server ? useRequestFetch() : $fetch
      const data = await fetch<Conversation>(`/api/conversations/${id}`)
      // updateLocally() only patches an existing entry (.map() no-ops if the
      // id isn't found) — a conversation opened directly (deep link,
      // bookmark, refresh before the list has loaded) isn't in the store
      // yet, so that would silently drop the fetched data. Insert it here
      // instead of routing through updateLocally, since this is a full
      // record, not a partial patch.
      const exists = conversations.value.some(c => c.id === id)
      conversations.value = exists
        ? replaceById(conversations.value, id, { ...conversations.value.find(c => c.id === id)!, ...data })
        : [data, ...conversations.value]
      return data
    } catch {
      return null
    }
  }

  async function create(overrides: Partial<Conversation> = {}): Promise<Conversation> {
    const { activeWorkspaceId } = useWorkspaces()
    const workspaceId = overrides.workspaceId || activeWorkspaceId.value
    if (!workspaceId) throw new Error('No active workspace')
    const data = await $fetch<Conversation>('/api/conversations', {
      method: 'POST',
      body: {
        workspaceId,
        title: overrides.title || 'New chat',
        modelId: overrides.modelId || settings.value.defaultModelId,
        mode: overrides.mode || 'chat',
        reasoningEffort: overrides.reasoningEffort,
        permissionMode: overrides.permissionMode,
        enabledToolIds: overrides.enabledToolIds
      }
    })
    conversations.value = [data, ...conversations.value]
    return data
  }

  function updateLocally(id: string, patch: Partial<Conversation>) {
    const current = conversations.value.find(c => c.id === id)
    if (current) conversations.value = replaceById(conversations.value, id, { ...current, ...patch, updatedAt: patch.updatedAt || Date.now() })
  }

  async function update(id: string, patch: Partial<Conversation>) {
    // Optimistic update
    updateLocally(id, patch)

    // Pick fields that are safe to update directly
    const apiPatch: Pick<Partial<Conversation>, 'title' | 'modelId' | 'mode' | 'permissionMode' | 'reasoningEffort' | 'enabledToolIds' | 'approvals'> = {}
    if (patch.title !== undefined) apiPatch.title = patch.title
    if (patch.modelId !== undefined) apiPatch.modelId = patch.modelId
    if (patch.mode !== undefined) apiPatch.mode = patch.mode
    if (patch.permissionMode !== undefined) apiPatch.permissionMode = patch.permissionMode
    if (patch.reasoningEffort !== undefined) apiPatch.reasoningEffort = patch.reasoningEffort
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
  async function remove(id: string) {
    conversations.value = removeById(conversations.value, id)
    await $fetch(`/api/conversations/${id}`, { method: 'DELETE' })
  }

  function setMessages(id: string, messages: UIMessage[]) {
    // We only update messages locally in useConversations because chat persistence
    // to DB happens in the chat stream itself / backend.
    updateLocally(id, { messages })
  }

  /**
   * Derive a title from the first user message. external MCP client does this server-side
   * with a model call; without a backend the first line is a good stand-in.
   */
  function titleFrom(text: string): string {
    const firstLine = text.trim().split('\n')[0] ?? ''
    const trimmed = firstLine.slice(0, 48).trim()
    if (!trimmed) return 'New chat'
    return firstLine.length > 48 ? `${trimmed}…` : trimmed
  }

  return { conversations, sorted, get, loadAll, loadOne, create, update, updateLocally, remove, setMessages, titleFrom }
}
