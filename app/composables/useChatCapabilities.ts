import type { ChatCapabilities } from '#shared/types/chat'

export function useChatCapabilities() {
  const capabilities = useState<ChatCapabilities>('chat-capabilities', () => ({
    terminal: { available: false }
  }))
  const loaded = useState<boolean>('chat-capabilities-loaded', () => false)

  async function load() {
    const fetch = import.meta.server ? useRequestFetch() : $fetch
    capabilities.value = await fetch<ChatCapabilities>('/api/chat/capabilities')
    loaded.value = true
    return capabilities.value
  }

  return { capabilities, loaded, load }
}
