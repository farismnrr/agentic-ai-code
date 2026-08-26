export interface AuthUseCases {
  findLoginUser: (email: string) => Promise<unknown>
  findUserForReauth: (userId: string) => Promise<unknown>
  findUserByEmail: (email: string) => Promise<unknown>
  userExists: (email: string) => Promise<boolean>
  createUser: (input: { email: string, name: string, passwordHash: string }) => Promise<unknown>
  addVerificationToken: (input: { tokenHash: string, userId: string, type: 'password_reset' | 'email_verify', expiresAt: Date }) => Promise<void>
  consumePasswordReset: (tokenHash: string, passwordHash: string) => Promise<unknown>
  consumeEmailVerification: (tokenHash: string) => Promise<unknown>
  requestEmailChange: (userId: string, input: { email: string, tokenHash: string, expiresAt: Date }) => Promise<unknown>
  consumeEmailChange: (tokenHash: string) => Promise<unknown>
  resendEmailVerification: (userId: string, input: { tokenHash: string, expiresAt: Date }) => Promise<unknown>
  hasActiveMfa: (userId: string) => Promise<boolean>
  verifyApiKey: (rawKey: string) => Promise<string>
}
