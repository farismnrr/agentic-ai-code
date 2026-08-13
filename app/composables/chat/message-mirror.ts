import type { UIMessage } from '#shared/types/chat'

export function createMessageMirror({ conversation, messages, setMessages }: { conversation: Ref<{ id: string } | undefined>, messages: { value: UIMessage[] }, setMessages: (id: string, messages: UIMessage[]) => void }) {
  let timer: ReturnType<typeof setTimeout>
  const flush = () => {
    clearTimeout(timer)
    if (conversation.value) setMessages(conversation.value.id, messages.value)
  }
  const schedule = () => {
    clearTimeout(timer)
    timer = setTimeout(flush, 300)
  }
  return { flush, schedule }
}
