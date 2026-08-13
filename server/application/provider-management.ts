export interface ProviderManagementCapabilities {
  listModelProviders: (...args: unknown[]) => unknown
  createModelProvider: (...args: unknown[]) => unknown
  updateModelProvider: (...args: unknown[]) => unknown
  deleteModelProvider: (...args: unknown[]) => unknown
  listProviderModelIds: (...args: unknown[]) => unknown
}
export const createProviderManagement = <T extends ProviderManagementCapabilities>(capabilities: T) => capabilities
