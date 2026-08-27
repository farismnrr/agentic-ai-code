type ModelConversation = {
  workspaceId: string
  modelId: string
  updatedAt: number
}

export function resolveNewChatModelId(input: {
  workspaceId: string | undefined
  conversations: ModelConversation[]
  validModelIds: Iterable<string>
  defaultModelId?: string | null
}) {
  if (!input.workspaceId) return undefined
  const valid = input.validModelIds instanceof Set ? input.validModelIds : new Set(input.validModelIds)
  const lastUsed = input.conversations
    .filter(conversation => conversation.workspaceId === input.workspaceId && valid.has(conversation.modelId))
    .sort((a, b) => b.updatedAt - a.updatedAt)[0]?.modelId

  if (lastUsed) return lastUsed
  if (input.defaultModelId && valid.has(input.defaultModelId)) return input.defaultModelId
  return undefined
}
