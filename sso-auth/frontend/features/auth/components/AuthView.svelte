<script lang="ts">
  import type { AuthState } from '../model/auth-state'
  import AuthenticatedPanel from './AuthenticatedPanel.svelte'
  import CallbackPanel from './CallbackPanel.svelte'
  import ErrorPanel from './ErrorPanel.svelte'
  import FlowPreview.svelte from './FlowPreview.svelte'
  import LoginPanel from './LoginPanel.svelte'
  import SessionExpiredPanel from './SessionExpiredPanel.svelte'

  export let state: AuthState
  export let onSignIn: () => void
  export let onSignOut: () => void
  export let onPreview: (state: AuthState) => void
</script>

<div class="hero min-h-screen px-4 py-10">
  <div class="hero-content flex w-full max-w-lg flex-col gap-4">
    {#if state.status === 'authenticated'}
      <AuthenticatedPanel user={state.user} {onSignOut} />
    {:else if state.status === 'callback_processing'}
      <CallbackPanel />
    {:else if state.status === 'session_expired'}
      <SessionExpiredPanel {onSignIn} />
    {:else if state.status === 'error'}
      <ErrorPanel message={state.message} {onSignIn} />
    {:else}
      <LoginPanel loading={state.status === 'loading'} {onSignIn} />
    {/if}

    <FlowPreview {onPreview} />
  </div>
</div>
