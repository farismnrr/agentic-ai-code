import type { ChatModel, Conversation } from '#shared/types/chat'

export function useConversationConfiguration(conversation: ComputedRef<Conversation | undefined>, models: Ref<ChatModel[]>) {
  const { update } = useConversations()
  const modelId = computed({
    get: () => conversation.value?.modelId ?? models.value[0]?.id,
    set: (value: string) => {
      if (conversation.value) update(conversation.value.id, { modelId: value })
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
  const enabledToolIds = computed({
    get: () => conversation.value?.enabledToolIds ?? [],
    set: (value: string[]) => {
      if (conversation.value) update(conversation.value.id, { enabledToolIds: value })
    }
  })
  return { modelId, mode, reasoningEffort, enabledToolIds }
}
