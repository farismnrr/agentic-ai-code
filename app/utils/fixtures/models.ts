import type { ChatModel } from '~/types/chat'

export const models: ChatModel[] = [
  {
    id: 'external-mcp-opus-5',
    label: 'Opus 5',
    description: 'Most capable. Best for hard reasoning and long tasks.',
    icon: 'i-lucide-sparkles'
  },
  {
    id: 'external-mcp-sonnet-5',
    label: 'Sonnet 5',
    description: 'Balanced speed and capability. Good default.',
    icon: 'i-lucide-zap'
  },
  {
    id: 'external-mcp-haiku-4-5',
    label: 'Haiku 4.5',
    description: 'Fastest and cheapest. Best for simple, high-volume work.',
    icon: 'i-lucide-feather'
  }
]

export const defaultModelId = 'external-mcp-sonnet-5'
