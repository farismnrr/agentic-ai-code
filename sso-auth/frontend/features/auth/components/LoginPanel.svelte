<script lang="ts">
  import AuthPanel from './AuthPanel.svelte'
  import GitHubSignInButton from './GitHubSignInButton.svelte'

  export let loading = false
  export let onSignIn: () => void
</script>

<AuthPanel
  eyebrow="Masih Awam"
  title="Sign in to Masih Awam"
  message="Authenticate with GitHub to continue to your developer workspace."
>
  <GitHubSignInButton {loading} {onSignIn} />

  <div class="mt-6 flex items-start gap-3 border-t border-slate-100 pt-5 text-xs leading-5 text-slate-500">
    <svg aria-hidden="true" viewBox="0 0 24 24" class="mt-0.5 size-4 shrink-0 fill-none stroke-current stroke-2">
      <rect x="5" y="10" width="14" height="10" rx="2"></rect>
      <path d="M8 10V7a4 4 0 0 1 8 0v3"></path>
    </svg>
    <p>Secure OAuth authentication. Review GitHub permissions before granting access.</p>
  </div>
</AuthPanel>
