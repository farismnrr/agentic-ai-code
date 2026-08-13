export interface ConversationRecord {
  id: string
  userId: string
  workspaceId: string
  title: string
  modelId: string
  mode: 'chat' | 'agent'
  reasoningEffort?: 'low' | 'medium' | 'high' | 'max' | null
  [key: string]: unknown
}

export interface ConversationPort {
  list(userId: string, workspaceId?: string): Promise<ConversationRecord[]>
  create(input: Omit<ConversationRecord, 'id' | 'userId'> & { userId: string }): Promise<ConversationRecord | undefined>
  update(userId: string, id: string, input: Record<string, unknown>): Promise<ConversationRecord | undefined>
  remove(userId: string, id: string): Promise<ConversationRecord | undefined>
  listMessages(userId: string, id: string): Promise<unknown>
  assertModelOwnership(userId: string, modelId: string): Promise<void>
  assertWorkspaceOwnership(userId: string, workspaceId: string): Promise<void>
}

export function createConversationUseCases(port: ConversationPort) {
  return {
    list: (userId: string, workspaceId?: string) => port.list(userId, workspaceId),
    create: async (input: Omit<ConversationRecord, 'id'> & { userId: string }) => {
      await port.assertModelOwnership(input.userId, input.modelId)
      await port.assertWorkspaceOwnership(input.userId, input.workspaceId)
      return port.create(input)
    },
    update: async (userId: string, id: string, input: Record<string, unknown>) => {
      if (typeof input.modelId === 'string') await port.assertModelOwnership(userId, input.modelId)
      return port.update(userId, id, input)
    },
    remove: (userId: string, id: string) => port.remove(userId, id),
    listMessages: (userId: string, id: string) => port.listMessages(userId, id)
  }
}
