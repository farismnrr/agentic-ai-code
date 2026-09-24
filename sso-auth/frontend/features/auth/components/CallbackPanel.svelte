<script lang="ts">
  import AuthPanel from './AuthPanel.svelte'

  export let phase: 'connecting' | 'callback' = 'callback'

  $: isConnecting = phase === 'connecting'
</script>

<AuthPanel
  title={isConnecting ? 'Connecting to GitHub' : 'Completing sign-in'}
  message={isConnecting
    ? 'Opening a secure GitHub authentication session.'
    : "We're verifying your GitHub account and preparing your session."}
>
  <div class="flex flex-col items-center py-2 text-center">
    <span class="loading loading-spinner loading-lg text-blue-600"></span>

    <div class="mt-7 w-full rounded-xl border border-slate-200 bg-slate-50 px-4 py-3">
      <div class="flex items-center justify-center gap-3 text-sm text-slate-600">
        <span class="loading loading-dots loading-sm text-blue-600"></span>
        <span>{isConnecting ? 'Connecting securely…' : 'Processing callback…'}</span>
      </div>
    </div>
  </div>
</AuthPanel>
