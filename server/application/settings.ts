import type { RequestTelemetryContext } from './observability/contracts'

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
export function createSettingsUseCases(port: Omit<SettingsPort, 'assertModelOwnership'>, assertModelOwnership: (userId: string, modelId: string) => Promise<void>, telemetry?: RequestTelemetryContext) {
  const span = <T>(operation: string, fn: () => Promise<T>) => telemetry ? telemetry.withSpan(operation, {}, fn) : fn()
  return {
    read: (userId: string, profile: { name?: string, email?: string }) => span('settings.get', () => port.read(userId, profile)),
    write: async (userId: string, updates: SettingsInput) => span('settings.update', async () => {
      if (updates.defaultModelId) await assertModelOwnership(userId, updates.defaultModelId)
      return port.write(userId, updates)
    })
  }
}
