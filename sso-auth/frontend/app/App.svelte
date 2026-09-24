<script lang="ts">
  import { AuthView, type AuthState } from '../features/auth'

  let authState: AuthState = { status: 'signed_out' }

  const demoUser = {
    login: 'farismnrr',
    avatarUrl: 'https://github.com/identicons/farismnrr.png'
  }

  function startSignIn() {
    authState = { status: 'loading' }

    window.setTimeout(() => {
      authState = { status: 'callback_processing' }

      window.setTimeout(() => {
        authState = { status: 'authenticated', user: demoUser }
      }, 900)
    }, 700)
  }

  function signOut() {
    authState = { status: 'signed_out' }
  }

  function previewState(state: AuthState) {
    authState = state
  }
</script>

<main class="min-h-screen bg-base-200 text-base-content">
  <AuthView
    state={authState}
    onSignIn={startSignIn}
    onSignOut={signOut}
    onPreview={previewState}
  />
</main>
