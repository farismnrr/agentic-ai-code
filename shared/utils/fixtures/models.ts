import type { ChatModel } from '#shared/types/chat'

export const models: ChatModel[] = [
  {
    id: 'claude-opus-5',
    label: 'Opus 5',
    description: 'Most capable. Best for hard reasoning and long tasks.',
    icon: 'i-lucide-sparkles'
  },
  {
    id: 'claude-sonnet-5',
    label: 'Sonnet 5',
    description: 'Balanced speed and capability. Good default.',
    icon: 'i-lucide-zap'
  },
  {
    id: 'claude-haiku-4-5',
    label: 'Haiku 4.5',
    description: 'Fastest and cheapest. Best for simple, high-volume work.',
    icon: 'i-lucide-feather'
  }
]

export const defaultModelId = 'claude-sonnet-5'
