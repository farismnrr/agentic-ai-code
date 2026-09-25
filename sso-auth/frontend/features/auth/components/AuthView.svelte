<script lang="ts">
  import type { AuthState } from '../model/auth-state'
  import AuthenticatedPanel from './AuthenticatedPanel.svelte'
  import CallbackPanel from './CallbackPanel.svelte'
  import ErrorPanel from './ErrorPanel.svelte'
  import LoginPanel from './LoginPanel.svelte'
  import SessionExpiredPanel from './SessionExpiredPanel.svelte'
  import AuthArtwork from '../layout/AuthArtwork.svelte'
  import BrandHeader from '../layout/BrandHeader.svelte'

  export let state: AuthState
  export let onSignIn: () => void
  export let onSignOut: () => void
</script>

<div class="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
  <BrandHeader />

  <div class="flex flex-1 items-center justify-center py-8 sm:py-12">
    <div class="w-full max-w-4xl">
      {#if state.status === 'signed_out'}
        <div class="grid items-center gap-10 lg:grid-cols-[minmax(0,1fr)_320px]">
          <LoginPanel loading={false} {onSignIn} />
          <AuthArtwork />
        </div>
      {:else if state.status === 'loading'}
        <div class="mx-auto max-w-md">
          <CallbackPanel phase="callback" />
        </div>
      {:else if state.status === 'authenticated'}
        <div class="mx-auto max-w-lg">
          <AuthenticatedPanel session={state.session} {onSignOut} />
        </div>
      {:else if state.status === 'session_expired'}
        <div class="mx-auto max-w-md">
          <SessionExpiredPanel {onSignIn} />
        </div>
      {:else if state.status === 'error'}
        <div class="mx-auto max-w-md">
          <ErrorPanel message={state.message} {onSignIn} />
        </div>
      {/if}
    </div>
  </div>

  <p class="pb-2 text-center text-xs text-slate-400">
    Secure authentication for Masih Awam services.
  </p>
</div>
