<script lang="ts">
  import AuthPanel from './AuthPanel.svelte'
  import GitHubSignInButton from './GitHubSignInButton.svelte'

  export let loading = false
  export let onSignIn: () => void
</script>

<AuthPanel
  badge="Masih Awam AI Code"
  badgeClass="border-primary/50 text-primary"
  title="Sign in"
  message="Continue with GitHub to access Masih Awam AI Code services."
>
  <GitHubSignInButton {loading} {onSignIn} />

  <p class="text-center text-sm text-base-content/70">
    Authentication is handled securely through GitHub.
  </p>
</AuthPanel>
