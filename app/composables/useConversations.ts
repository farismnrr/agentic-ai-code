import type { Conversation, UIMessage } from '~/types/chat'
import { seedConversations } from '~/utils/fixtures/conversations'
import { defaultEnabledToolIds } from '~/utils/fixtures/mcp-servers'
import { defaultModelId } from '~/utils/fixtures/models'

/**
 * In-memory conversation store.
 *
 * Backed by `useState` rather than a module-scope `ref`: on the server a
 * module-scope ref is shared across every request, so one visitor's
 * conversations would leak into another's. State resets on reload by design —
 * persistence is out of scope for this iteration.
 */
export function useConversations() {
  const conversations = useState<Conversation[]>('conversations', () => [...seedConversations])

  /** Newest first, which is the order the sidebar renders. */
  const sorted = computed(() =>
    [...conversations.value].sort((a, b) => b.updatedAt - a.updatedAt)
  )

  function get(id: string): Conversation | undefined {
    return conversations.value.find(c => c.id === id)
  }

  function create(overrides: Partial<Conversation> = {}): Conversation {
    const now = Date.now()
    const conversation: Conversation = {
      id: `c_${now.toString(36)}_${Math.random().toString(36).slice(2, 7)}`,
      title: 'New chat',
      createdAt: now,
      updatedAt: now,
      messages: [],
      modelId: defaultModelId,
      enabledToolIds: [...defaultEnabledToolIds],
      approvals: {},
      ...overrides
    }
    conversations.value = [conversation, ...conversations.value]
    return conversation
  }

  function update(id: string, patch: Partial<Conversation>) {
    conversations.value = conversations.value.map(c =>
      c.id === id ? { ...c, ...patch, updatedAt: Date.now() } : c
    )
  }

  function remove(id: string) {
    conversations.value = conversations.value.filter(c => c.id !== id)
  }

  function setMessages(id: string, messages: UIMessage[]) {
    update(id, { messages })
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

  return { conversations, sorted, get, create, update, remove, setMessages, titleFrom }
}
