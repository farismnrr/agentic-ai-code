<script lang="ts">
  import GitHubSignInButton from './GitHubSignInButton.svelte'

  export let loading = false
  export let errorMessage: string | undefined = undefined
</script>

<section class="card w-full border border-base-300 bg-base-100 shadow-xl">
  <div class="card-body gap-6 p-8 sm:p-10">
    <header class="space-y-3">
      <div class="badge badge-neutral badge-outline">Masih Awam AI Code</div>
      <div class="space-y-2">
        <h1 class="card-title text-3xl font-bold">Sign in</h1>
        <p class="text-base-content/70">
          Continue with GitHub to access Masih Awam AI Code services.
        </p>
      </div>
    </header>

    {#if errorMessage}
      <div class="alert alert-error" role="alert">
        <span>{errorMessage}</span>
      </div>
    {/if}

    <GitHubSignInButton {loading} />

    <p class="text-center text-xs text-base-content/50">
      Authentication is handled securely through GitHub.
    </p>
  </div>
</section>
