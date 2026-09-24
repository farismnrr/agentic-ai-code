<script lang="ts">
  import type { AuthState } from '../model/auth-state'
  import AuthenticatedPanel from './AuthenticatedPanel.svelte'
  import LoginPanel from './LoginPanel.svelte'

  export let state: AuthState
</script>

<div class="hero min-h-screen px-4 py-10">
  <div class="hero-content w-full max-w-lg">
    {#if state.status === 'authenticated'}
      <AuthenticatedPanel user={state.user} />
    {:else}
      <LoginPanel
        loading={state.status === 'loading'}
        errorMessage={state.status === 'error' ? state.message : undefined}
      />
    {/if}
  </div>
</div>
