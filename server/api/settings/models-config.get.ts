import { PROVIDER_TYPE_OPTIONS } from '#shared/utils/providers'
import type { ChatModel, ModelProvider } from '#shared/types/chat'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)

  const [providers, models] = await Promise.all([
    event.context.application.providers.list(session.user.id) as Promise<ModelProvider[]>,
    event.context.application.models.list(session.user.id) as Promise<ChatModel[]>
  ])

  return {
    providers,
    models,
    providerTypes: PROVIDER_TYPE_OPTIONS,
    iconOptions: [
      { label: 'Sparkles', value: 'i-lucide-sparkles', icon: 'i-lucide-sparkles' },
      { label: 'Bot', value: 'i-lucide-bot', icon: 'i-lucide-bot' },
      { label: 'Box', value: 'i-lucide-box', icon: 'i-lucide-box' },
      { label: 'CPU', value: 'i-lucide-cpu', icon: 'i-lucide-cpu' },
      { label: 'Message', value: 'i-lucide-message-square', icon: 'i-lucide-message-square' },
      { label: 'Zap', value: 'i-lucide-zap', icon: 'i-lucide-zap' },
      { label: 'Brain', value: 'i-lucide-brain', icon: 'i-lucide-brain' },
      { label: 'Terminal', value: 'i-lucide-terminal', icon: 'i-lucide-terminal' }
    ]
  }
})
