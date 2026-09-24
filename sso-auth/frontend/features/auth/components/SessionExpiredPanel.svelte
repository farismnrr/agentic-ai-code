<script lang="ts">
  import AuthPanel from './AuthPanel.svelte'
  import GitHubSignInButton from './GitHubSignInButton.svelte'

  export let onSignIn: () => void
</script>

<AuthPanel
  badge="Session expired"
  badgeClass="border-warning/60 text-warning"
  title="Sign in again"
  message="Your session is no longer active. Continue with GitHub to start a new session."
>
  <GitHubSignInButton {onSignIn} />
</AuthPanel>
