export interface ModelPort<Model, Input, Update> {
  list(userId: string): Promise<Model[]>
  create(userId: string, providerId: string, input: Input): Promise<Model>
  update(userId: string, id: string, updates: Update): Promise<Model>
  remove(userId: string, id: string): Promise<{ ok: true }>
}

export function createModelUseCases<Model, Input, Update>(port: ModelPort<Model, Input, Update>) {
  return {
    list: (userId: string) => port.list(userId),
    create: (userId: string, providerId: string, input: Input) => port.create(userId, providerId, input),
    update: (userId: string, id: string, updates: Update) => port.update(userId, id, updates),
    remove: (userId: string, id: string) => port.remove(userId, id)
  }
}
