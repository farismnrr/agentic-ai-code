import type { RequestTelemetryContext } from './observability/contracts'

export interface ModelPort<Model, Input, Update> {
  list(userId: string): Promise<Model[]>
  create(userId: string, providerId: string, input: Input): Promise<Model>
  update(userId: string, id: string, updates: Update): Promise<Model>
  remove(userId: string, id: string): Promise<{ ok: true }>
}

export function createModelUseCases<Model, Input, Update>(port: ModelPort<Model, Input, Update>, telemetry?: RequestTelemetryContext) {
  const span = <T>(operation: string, fn: () => Promise<T>) => telemetry ? telemetry.withSpan(operation, {}, fn) : fn()
  return {
    list: (userId: string) => span('model.list', () => port.list(userId)),
    create: (userId: string, providerId: string, input: Input) => span('model.create', () => port.create(userId, providerId, input)),
    update: (userId: string, id: string, updates: Update) => span('model.update', () => port.update(userId, id, updates)),
    remove: (userId: string, id: string) => span('model.delete', () => port.remove(userId, id))
  }
}
