import type { RequestTelemetryContext } from './observability/contracts'

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

export function createConversationUseCases(port: ConversationPort, telemetry?: RequestTelemetryContext) {
  const span = <T>(operation: string, fn: () => Promise<T>) => telemetry ? telemetry.withSpan(operation, {}, fn) : fn()
  return {
    list: (userId: string, workspaceId?: string) => span('conversation.list', () => port.list(userId, workspaceId)),
    create: async (input: Omit<ConversationRecord, 'id'> & { userId: string }) => span('conversation.create', async () => {
      await port.assertModelOwnership(input.userId, input.modelId)
      await port.assertWorkspaceOwnership(input.userId, input.workspaceId)
      return port.create(input)
    }),
    update: async (userId: string, id: string, input: Record<string, unknown>) => span('conversation.update', async () => {
      if (typeof input.modelId === 'string') await port.assertModelOwnership(userId, input.modelId)
      return port.update(userId, id, input)
    }),
    remove: (userId: string, id: string) => span('conversation.delete', () => port.remove(userId, id)),
    listMessages: (userId: string, id: string) => span('conversation.get', () => port.listMessages(userId, id))
  }
}
