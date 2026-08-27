import type { Conversation } from '#shared/types/chat'

export function useConversationConfiguration(conversation: ComputedRef<Conversation | undefined>) {
  const { update } = useConversations()
  const modelId = computed({
    get: () => conversation.value?.modelId,
    set: (value: string | undefined) => {
      if (conversation.value && value) update(conversation.value.id, { modelId: value })
    }
  })
  const mode = computed({
    get: () => conversation.value?.mode ?? 'chat',
    set: (value: Conversation['mode']) => {
      if (conversation.value) update(conversation.value.id, { mode: value })
    }
  })
  const reasoningEffort = computed({
    get: () => conversation.value?.reasoningEffort ?? 'medium',
    set: (value: NonNullable<Conversation['reasoningEffort']>) => {
      if (conversation.value) update(conversation.value.id, { reasoningEffort: value })
    }
  })
  const permissionMode = computed({
    get: () => conversation.value?.permissionMode ?? 'manual',
    set: (value: Conversation['permissionMode']) => {
      if (conversation.value) update(conversation.value.id, { permissionMode: value })
    }
  })
  return { modelId, mode, reasoningEffort, permissionMode }
}
