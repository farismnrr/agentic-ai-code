export interface SettingsInput {
  language?: string
  streaming?: boolean
  sendOnEnter?: boolean
  defaultModelId?: string | null
  temperature?: number
  systemPrompt?: string
  displayName?: string
  email?: string
}
export interface SettingsPort {
  read(userId: string, profile: { name?: string, email?: string }): Promise<unknown>
  write(userId: string, updates: SettingsInput): Promise<unknown>
}
export function createSettingsUseCases(port: Omit<SettingsPort, 'assertModelOwnership'>, assertModelOwnership: (userId: string, modelId: string) => Promise<void>) {
  return {
    read: (userId: string, profile: { name?: string, email?: string }) => port.read(userId, profile),
    write: async (userId: string, updates: SettingsInput) => {
      if (updates.defaultModelId) await assertModelOwnership(userId, updates.defaultModelId)
      return port.write(userId, updates)
    }
  }
}
