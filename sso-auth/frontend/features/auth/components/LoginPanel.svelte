<script lang="ts">
  import AuthPanel from './AuthPanel.svelte'
  import GitHubSignInButton from './GitHubSignInButton.svelte'

  export let loading = false
</script>

<AuthPanel
  badge="Masih Awam AI Code"
  title="Sign in"
  message="Continue with GitHub to access Masih Awam AI Code services."
>
  <GitHubSignInButton {loading} />

  <p class="text-center text-sm text-base-content/65">
    Authentication is handled securely through GitHub.
  </p>
</AuthPanel>
