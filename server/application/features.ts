export interface FeatureCapabilities {
  getSettings: (...args: unknown[]) => unknown
  updateSettings: (...args: unknown[]) => unknown
}
export const createFeatures = <T extends FeatureCapabilities>(capabilities: T) => capabilities
