import type { RequestTelemetryContext } from './observability/contracts'

export interface ProviderManagementPort<Input, Update, Result> {
  list(userId: string): Promise<Result[]>
  create(userId: string, input: Input): Promise<Result>
  update(userId: string, id: string, updates: Update): Promise<Result>
  remove(userId: string, id: string): Promise<{ ok: true }>
  discoverModels(userId: string, id: string): Promise<string[]>
}

export function createProviderManagementUseCases<Input, Update, Result>(port: ProviderManagementPort<Input, Update, Result>, telemetry?: RequestTelemetryContext) {
  const span = <T>(operation: string, fn: () => Promise<T>) => telemetry ? telemetry.withSpan(operation, {}, fn) : fn()
  return {
    list: (userId: string) => span('provider.list', () => port.list(userId)),
    create: (userId: string, input: Input) => span('provider.create', () => port.create(userId, input)),
    update: (userId: string, id: string, updates: Update) => span('provider.update', () => port.update(userId, id, updates)),
    remove: (userId: string, id: string) => span('provider.delete', () => port.remove(userId, id)),
    discoverModels: (userId: string, id: string) => span('provider.discover_models', () => port.discoverModels(userId, id))
  }
}
