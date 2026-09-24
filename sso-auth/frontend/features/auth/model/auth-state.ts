export type AuthenticatedUser = {
  login: string
  avatarUrl?: string
}

export type AuthState =
  | { status: 'signed_out' }
  | { status: 'loading' }
  | { status: 'callback_processing' }
  | { status: 'authenticated'; user: AuthenticatedUser }
  | { status: 'session_expired' }
  | { status: 'error'; message: string }
