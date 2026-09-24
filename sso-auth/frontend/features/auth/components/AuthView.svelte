<script lang="ts">
  import type { AuthState } from '../model/auth-state'
  import AuthenticatedPanel from './AuthenticatedPanel.svelte'
  import CallbackPanel from './CallbackPanel.svelte'
  import ErrorPanel from './ErrorPanel.svelte'
  import LoginPanel from './LoginPanel.svelte'
  import SessionExpiredPanel from './SessionExpiredPanel.svelte'

  export let state: AuthState
</script>

<div class="hero min-h-screen px-4 py-10">
  <div class="hero-content w-full max-w-lg">
    {#if state.status === 'authenticated'}
      <AuthenticatedPanel user={state.user} />
    {:else if state.status === 'callback_processing'}
      <CallbackPanel />
    {:else if state.status === 'session_expired'}
      <SessionExpiredPanel />
    {:else if state.status === 'error'}
      <ErrorPanel message={state.message} />
    {:else}
      <LoginPanel loading={state.status === 'loading'} />
    {/if}
  </div>
</div>
