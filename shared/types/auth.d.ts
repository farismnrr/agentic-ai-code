declare module '#auth-utils' {
  interface SecureSessionData {
    authSession?: {
      id: string
      secret: string
      issuedAt: number
      freshAuthAt: number
    }
    authMethod?: 'api_key'
  }

  interface User {
    id: string
    email?: string
    name?: string
    avatarUrl?: string | null
    emailVerifiedAt?: string | null
    authVersion?: number
    role?: 'user' | 'admin'
  }
}

export {}
