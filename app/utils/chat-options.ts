import type { ChatModel, Conversation } from '#shared/types/chat'

export const chatModeItems = [
  { label: 'Chat Mode', value: 'chat', icon: 'i-lucide-message-square' },
  { label: 'Agent Mode', value: 'agent', icon: 'i-lucide-bot' }
] satisfies Array<{ label: string, value: Conversation['mode'], icon: string }>

export const reasoningEffortItems = [
  { label: 'Low Effort', value: 'low' },
  { label: 'Medium Effort', value: 'medium' },
  { label: 'High Effort', value: 'high' },
  { label: 'Max Effort', value: 'max' }
] satisfies Array<{ label: string, value: NonNullable<Conversation['reasoningEffort']> }>

export function modelSupportsReasoning(model: Pick<ChatModel, 'thinkingEnabled'> | undefined) {
  return model?.thinkingEnabled ?? false
}
