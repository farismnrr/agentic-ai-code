import { defaultModelId } from '#shared/utils/fixtures/models'

export interface AppSettings {
  language: string
  streaming: boolean
  sendOnEnter: boolean
  defaultModelId: string
  temperature: number
  systemPrompt: string
  displayName: string
  email: string
}

export function useSettings() {
  const settings = useState<AppSettings>('settings', () => ({
    language: 'en',
    streaming: true,
    sendOnEnter: true,
    defaultModelId,
    temperature: 0.7,
    systemPrompt: '',
    displayName: 'Faris',
    email: 'farismunir2@gmail.com'
  }))

  async function load() {
    const data = await $fetch<AppSettings>('/api/settings')
    settings.value = data
  }

  async function update(patch: Partial<AppSettings>) {
    settings.value = { ...settings.value, ...patch }
    const data = await $fetch<AppSettings>('/api/settings', {
      method: 'PUT',
      body: patch
    })
    settings.value = data
  }

  return Object.assign(settings, { load, update })
}
