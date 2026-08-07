import type { ChatModel } from '#shared/types/chat'

export const models: ChatModel[] = [
  {
    id: 'high-thinking-models',
    label: 'High Thinking',
    description: 'Most capable. Best for hard reasoning and long tasks.',
    icon: 'i-lucide-sparkles'
  },
  {
    id: 'vx/gemini-3-flash-preview',
    label: 'Flash Preview',
    description: 'Balanced speed and capability. Good default.',
    icon: 'i-lucide-zap'
  },
  {
    id: 'free-models',
    label: 'Free Models',
    description: 'Fastest and cheapest. Best for simple, high-volume work.',
    icon: 'i-lucide-feather'
  }
]

export const defaultModelId = 'vx/gemini-3-flash-preview'
