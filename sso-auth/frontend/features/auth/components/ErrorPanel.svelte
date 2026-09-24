<script lang="ts">
  import AuthPanel from './AuthPanel.svelte'
  import GitHubSignInButton from './GitHubSignInButton.svelte'

  export let message: string
</script>

<AuthPanel
  badge="Authentication error"
  badgeClass="badge-error badge-outline"
  title="We couldn't sign you in"
  {message}
>
  <div class="card-actions">
    <GitHubSignInButton />
  </div>
</AuthPanel>
