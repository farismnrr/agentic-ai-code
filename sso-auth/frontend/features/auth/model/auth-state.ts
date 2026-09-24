export type AuthenticatedUser = {
  login: string
  avatarUrl?: string
}

export type AuthState =
  | { status: 'signed_out' }
  | { status: 'loading' }
  | { status: 'authenticated'; user: AuthenticatedUser }
  | { status: 'error'; message: string }
