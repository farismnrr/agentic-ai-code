/**
 * Carries the first prompt from the empty state into the conversation page.
 *
 * The empty state creates the conversation and routes away before a chat
 * instance exists, so the text has to survive one navigation. Passing it as a
 * query param would put user input in the URL and in history; this keeps it in
 * memory for exactly one read.
 */
export function usePendingPrompt() {
  const pending = useState<Record<string, string>>('pending-prompts', () => ({}))

  function set(conversationId: string, text: string) {
    pending.value = { ...pending.value, [conversationId]: text }
  }

  /** Returns the prompt and clears it, so a refresh doesn't re-send. */
  function take(conversationId: string): string | undefined {
    const text = pending.value[conversationId]
    if (text === undefined) return undefined
    const { [conversationId]: _, ...rest } = pending.value
    pending.value = rest
    return text
  }

  return { set, take }
}
