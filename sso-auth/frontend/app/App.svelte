<script lang="ts">
  import AuthView from '../features/auth/components/AuthView.svelte'
  import type { AuthState } from '../features/auth/model/auth-state'

  const authState: AuthState = { status: 'signed_out' }
</script>

<main class="min-h-screen bg-base-200">
  <AuthView state={authState} />
</main>
