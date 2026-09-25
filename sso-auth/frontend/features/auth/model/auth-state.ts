export type AuthenticatedUser = {
  id: number
  login: string
  avatarUrl?: string
}

export type AuthenticatedSession = {
  user: AuthenticatedUser
  issuedAt: number
  expiresAt: number
}

export type AuthState =
  | { status: 'loading' }
  | { status: 'signed_out' }
  | { status: 'authenticated'; session: AuthenticatedSession }
  | { status: 'session_expired' }
  | { status: 'error'; message: string }
