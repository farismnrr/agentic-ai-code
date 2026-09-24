<script lang="ts">
  import AuthPanel from './AuthPanel.svelte'
  import GitHubSignInButton from './GitHubSignInButton.svelte'
</script>

<AuthPanel
  badge="Session expired"
  badgeClass="badge-warning badge-outline"
  title="Sign in again"
  message="Your session is no longer active. Continue with GitHub to start a new session."
>
  <GitHubSignInButton />
</AuthPanel>
