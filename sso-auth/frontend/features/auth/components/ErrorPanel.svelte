<script lang="ts">
  import AuthPanel from './AuthPanel.svelte'
  import GitHubSignInButton from './GitHubSignInButton.svelte'

  export let message: string
  export let onSignIn: () => void
</script>

<AuthPanel
  badge="Authentication error"
  badgeClass="border-error/60 text-error"
  title="We couldn't sign you in"
  {message}
>
  <GitHubSignInButton {onSignIn} />
</AuthPanel>
