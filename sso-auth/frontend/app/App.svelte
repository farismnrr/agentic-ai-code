<script lang="ts">
  import { AuthView, type AuthState } from '../features/auth'

  const authState: AuthState = { status: 'signed_out' }
</script>

<main class="min-h-screen bg-base-200">
  <AuthView state={authState} />
</main>
