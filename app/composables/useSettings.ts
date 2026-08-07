import { defaultModelId } from '~/utils/fixtures/models'

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

/**
 * App-wide preferences. In-memory like everything else this iteration —
 * persistence is the backend's job and is deliberately out of scope.
 */
export function useSettings() {
  return useState<AppSettings>('settings', () => ({
    language: 'en',
    streaming: true,
    sendOnEnter: true,
    defaultModelId,
    temperature: 0.7,
    systemPrompt: '',
    displayName: 'Faris',
    email: 'farismunir2@gmail.com'
  }))
}
