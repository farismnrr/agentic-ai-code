import type { RequestTelemetryContext } from './observability/contracts'

export interface WorkspacePort<Workspace> {
  list(userId: string): Promise<Workspace[]>
  create(userId: string, name: string, path: string): Promise<Workspace>
  update(userId: string, id: string, name: string, path?: string): Promise<Workspace>
  remove(userId: string, id: string): Promise<{ ok: true }>
  find(userId: string, id: string): Promise<Workspace>
  setActive(userId: string, id: string | null): Promise<void>
}

export function createWorkspaceUseCases<Workspace>(port: WorkspacePort<Workspace>, telemetry?: RequestTelemetryContext) {
  const span = <T>(operation: string, fn: () => Promise<T>) => telemetry ? telemetry.withSpan(operation, {}, fn) : fn()
  return {
    list: (userId: string) => span('workspace.list', () => port.list(userId)),
    create: (userId: string, name: string, path: string) => span('workspace.create', () => port.create(userId, name, path)),
    update: (userId: string, id: string, name: string, path?: string) => span('workspace.update', () => port.update(userId, id, name, path)),
    remove: (userId: string, id: string) => span('workspace.delete', () => port.remove(userId, id)),
    setActive: async (userId: string, id: string | null) => span('workspace.set_active', async () => {
      if (id !== null) await port.find(userId, id)
      await port.setActive(userId, id)
    })
  }
}
