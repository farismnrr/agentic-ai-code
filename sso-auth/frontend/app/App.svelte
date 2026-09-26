<script lang="ts">
  import { onMount } from 'svelte'

  import { AuthView, type AuthState } from '../features/auth'
  import { ConnectionConsent } from '../features/connection-consent'
  import AgentationInspector from '../shared/dev/AgentationInspector.svelte'

  type SessionPayload = {
    authenticated: boolean
    user?: {
      id: number
      login: string
      avatar_url?: string
    }
    issuedAt?: number
    expiresAt?: number
  }

  let authState: AuthState = { status: 'loading' }
  const routePath = window.location.pathname

  onMount(() => {
    if (routePath !== '/connect') void hydrateSession()
  })

  async function hydrateSession() {
    if (new URL(window.location.href).searchParams.get('auth_error')) {
      authState = {
        status: 'error',
        message: "We couldn't complete GitHub sign-in. Please try again."
      }
      window.history.replaceState({}, '', window.location.pathname)
      return
    }

    try {
      const response = await fetch('/api/session', {
        headers: { Accept: 'application/json' },
        credentials: 'same-origin'
      })
      if (!response.ok) throw new Error('session request failed')

      const payload = await response.json() as SessionPayload
      if (
        payload.authenticated
        && payload.user
        && typeof payload.issuedAt === 'number'
        && typeof payload.expiresAt === 'number'
      ) {
        authState = {
          status: 'authenticated',
          session: {
            user: {
              id: payload.user.id,
              login: payload.user.login,
              avatarUrl: payload.user.avatar_url
            },
            issuedAt: payload.issuedAt,
            expiresAt: payload.expiresAt
          }
        }
        return
      }

      authState = { status: 'signed_out' }
    } catch {
      authState = {
        status: 'error',
        message: 'Unable to check your authentication session. Please try again.'
      }
    }
  }

  function startSignIn() {
    window.location.assign('/auth/github')
  }

  async function signOut() {
    authState = { status: 'loading' }

    try {
      const response = await fetch('/auth/logout', {
        method: 'POST',
        credentials: 'same-origin'
      })
      if (!response.ok) throw new Error('logout failed')
      authState = { status: 'signed_out' }
    } catch {
      authState = {
        status: 'error',
        message: 'Unable to sign out. Please try again.'
      }
    }
  }
</script>

<main class="min-h-screen bg-white text-slate-950">
  {#if routePath === '/connect'}
    <ConnectionConsent />
  {:else}
    <AuthView state={authState} onSignIn={startSignIn} onSignOut={signOut} />
  {/if}
</main>

<AgentationInspector />
