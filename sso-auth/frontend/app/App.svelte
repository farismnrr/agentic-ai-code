<script lang="ts">
  import { AuthView, type AuthState } from '../features/auth'

  const authState: AuthState = { status: 'signed_out' }
</script>

<main class="min-h-screen bg-base-200 text-base-content">
  <AuthView state={authState} />
</main>
