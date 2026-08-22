declare module '#auth-utils' {
  interface User {
    id: string
    email?: string
    name?: string
    avatarUrl?: string | null
    emailVerifiedAt?: string | null
    authVersion?: number
  }
}

export {}
