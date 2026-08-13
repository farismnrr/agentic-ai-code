export interface AccountDataCapabilities { [operation: string]: (...args: unknown[]) => unknown }
export const createAccountData = <T extends AccountDataCapabilities>(capabilities: T) => capabilities
