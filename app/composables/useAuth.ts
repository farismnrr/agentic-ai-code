export interface SessionUser {
  name: string
  email: string
}

const STORAGE_KEY = 'ai-code.session'

/**
 * Fake authentication for the prototype. Any credentials are accepted —
 * there is no backend to check them against.
 *
 * The session is the one thing that survives a reload. Everything else
 * (conversations, settings, MCP servers) deliberately resets to seed data, so
 * each demo starts clean. Without this exception the route guard would bounce
 * you to /login on every refresh, which reads as broken rather than clean.
 */
export function useAuth() {
  const user = useState<SessionUser | null>('auth-user', () => null)

  /** Called once from a client plugin — localStorage doesn't exist during SSR. */
  function restore() {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return
    try {
      user.value = JSON.parse(raw) as SessionUser
    } catch {
      // A corrupt entry shouldn't lock anyone out of the prototype.
      localStorage.removeItem(STORAGE_KEY)
    }
  }

  function persist(value: SessionUser | null) {
    if (import.meta.server) return
    if (value) localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
    else localStorage.removeItem(STORAGE_KEY)
  }

  /** Derives a display name from an email so the UI has something to show. */
  function nameFromEmail(email: string): string {
    const local = email.split('@')[0] ?? 'there'
    return local
      .split(/[._-]+/)
      .filter(Boolean)
      .map(part => part.charAt(0).toUpperCase() + part.slice(1))
      .join(' ') || 'there'
  }

  function login(email: string, name?: string) {
    const session: SessionUser = { name: name?.trim() || nameFromEmail(email), email }
    user.value = session
    persist(session)
    return session
  }

  /** Same effect as login — there's no account store to write to. */
  const register = login

  function logout() {
    user.value = null
    persist(null)
  }

  const isAuthenticated = computed(() => user.value !== null)

  return { user, isAuthenticated, restore, login, register, logout }
}
