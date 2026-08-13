export interface AccountDataUseCases {
  listSidebarData: (userId: string) => Promise<[unknown[], unknown[]]>
  generateApiKey: () => { rawKey: string, keyPrefix: string, keyHash: string }
  listUserDevices: (userId: string) => Promise<unknown>
  registerUserDevice: (input: { userId: string, name: string, fingerprint: string }) => Promise<unknown>
  revokeDevice: (userId: string, id: string) => Promise<unknown[]>
  listApiKeys: (userId: string) => Promise<unknown>
  createApiKey: (input: { userId: string, name: string, keyHash: string, keyPrefix: string }) => Promise<unknown[]>
  deleteApiKey: (userId: string, id: string) => Promise<unknown[]>
}
