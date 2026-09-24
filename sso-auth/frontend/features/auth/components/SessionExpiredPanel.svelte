<script lang="ts">
  import AuthPanel from './AuthPanel.svelte'
  import GitHubSignInButton from './GitHubSignInButton.svelte'

  export let onSignIn: () => void
</script>

<AuthPanel
  title="Session expired"
  message="For your security, your session has ended. Sign in again to continue."
>
  <div class="mb-6 flex justify-center">
    <div class="grid size-14 place-items-center rounded-full border border-amber-200 bg-amber-50 text-2xl font-semibold text-amber-600">
      !
    </div>
  </div>

  <GitHubSignInButton label="Sign in again" {onSignIn} />
</AuthPanel>
