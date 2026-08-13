export interface WorkspacePort<Workspace> {
  list(userId: string): Promise<Workspace[]>
  create(userId: string, name: string, path: string): Promise<Workspace>
  update(userId: string, id: string, name: string, path?: string): Promise<Workspace>
  remove(userId: string, id: string): Promise<{ ok: true }>
  find(userId: string, id: string): Promise<Workspace>
  setActive(userId: string, id: string | null): Promise<void>
}

export function createWorkspaceUseCases<Workspace>(port: WorkspacePort<Workspace>) {
  return {
    list: (userId: string) => port.list(userId),
    create: (userId: string, name: string, path: string) => port.create(userId, name, path),
    update: (userId: string, id: string, name: string, path?: string) => port.update(userId, id, name, path),
    remove: (userId: string, id: string) => port.remove(userId, id),
    setActive: async (userId: string, id: string | null) => {
      if (id !== null) await port.find(userId, id)
      await port.setActive(userId, id)
    }
  }
}
