export interface AuthCapabilities { [operation: string]: (...args: unknown[]) => unknown }
export const createAuth = <T extends AuthCapabilities>(capabilities: T) => capabilities
