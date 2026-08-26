export interface AccountDataUseCases {
  listSidebarData: (userId: string) => Promise<[unknown[], unknown[]]>
  generateApiKey: () => { rawKey: string, keyPrefix: string, keyHash: string }
  listUserDevices: (userId: string) => Promise<unknown>
  registerUserDevice: (input: { userId: string, name: string, fingerprint: string }) => Promise<unknown>
  revokeDevice: (userId: string, id: string) => Promise<unknown[]>
  listApiKeys: (userId: string) => Promise<unknown>
  createApiKey: (input: { userId: string, name: string, keyHash: string, keyPrefix: string }) => Promise<unknown[]>
  deleteApiKey: (userId: string, id: string) => Promise<unknown[]>
  createAuthSession: (input: { id: string, userId: string, secretHash: string }) => Promise<unknown>
  listAuthSessions: (userId: string) => Promise<unknown[]>
  validateAuthSession: (input: { id: string, userId: string, secretHash: string }) => Promise<boolean>
  touchAuthSession: (input: { id: string, userId: string, secretHash: string }) => Promise<void>
  revokeAuthSession: (userId: string, id: string) => Promise<unknown[]>
  revokeOtherAuthSessions: (userId: string, currentId: string) => Promise<unknown[]>
  getUserRole: (userId: string) => Promise<'user' | 'admin' | undefined>
}
