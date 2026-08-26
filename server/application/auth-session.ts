import { createHash, randomBytes, randomUUID } from 'node:crypto'

export const FRESH_AUTH_MAX_AGE_MS = 10 * 60 * 1000

export type AuthSessionUser = {
  id: string
  email?: string
  name?: string
  avatarUrl?: string | null
  emailVerifiedAt?: string | null
  authVersion?: number
  role?: 'user' | 'admin'
}

export type BrowserAuthSession = {
  id: string
  secret: string
  issuedAt: number
  freshAuthAt: number
}

export function issueBrowserAuthSession(): BrowserAuthSession {
  const now = Date.now()
  return {
    id: randomUUID(),
    secret: randomBytes(32).toString('hex'),
    issuedAt: now,
    freshAuthAt: now
  }
}

export function hashSessionSecret(secret: string) {
  return createHash('sha256').update(secret).digest('hex')
}

export function isFreshAuth(session: unknown, now = Date.now()) {
  if (!isRecord(session) || !isRecord(session.secure) || !isRecord(session.secure.authSession)) return false
  const freshAuthAt = session.secure.authSession.freshAuthAt
  return typeof freshAuthAt === 'number'
    && freshAuthAt > 0
    && now - freshAuthAt >= 0
    && now - freshAuthAt <= FRESH_AUTH_MAX_AGE_MS
}

export function browserSessionFrom(session: unknown) {
  if (!isRecord(session) || !isRecord(session.secure) || !isRecord(session.secure.authSession)) return undefined
  const authSession = session.secure.authSession
  if (authSession.type === 'api_key' || typeof authSession.id !== 'string' || typeof authSession.secret !== 'string' || typeof authSession.issuedAt !== 'number') return undefined
  return {
    id: authSession.id,
    secret: authSession.secret,
    issuedAt: authSession.issuedAt,
    freshAuthAt: typeof authSession.freshAuthAt === 'number' ? authSession.freshAuthAt : 0
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
