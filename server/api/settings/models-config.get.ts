import { listModelProviders } from '../../utils/providers'
import { listModels } from '../../utils/models'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)

  const [providers, models] = await Promise.all([
    listModelProviders(session.user.id),
    listModels(session.user.id)
  ])

  return {
    providers,
    models,
    providerTypes: ['9router', 'gcp_agent_platform'],
    providerOptions: providers.map(p => ({ label: p.name, value: p.id })),
    modelItems: models.map(m => ({ label: m.label, value: m.id, icon: 'i-lucide-box' })),
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
