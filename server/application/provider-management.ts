export interface ProviderManagementPort<Input, Update, Result> {
  list(userId: string): Promise<Result[]>
  create(userId: string, input: Input): Promise<Result>
  update(userId: string, id: string, updates: Update): Promise<Result>
  remove(userId: string, id: string): Promise<{ ok: true }>
  discoverModels(userId: string, id: string): Promise<string[]>
}

export function createProviderManagementUseCases<Input, Update, Result>(port: ProviderManagementPort<Input, Update, Result>) {
  return {
    list: (userId: string) => port.list(userId),
    create: (userId: string, input: Input) => port.create(userId, input),
    update: (userId: string, id: string, updates: Update) => port.update(userId, id, updates),
    remove: (userId: string, id: string) => port.remove(userId, id),
    discoverModels: (userId: string, id: string) => port.discoverModels(userId, id)
  }
}
