<script lang="ts">
  import GitHubIcon from '../../../shared/components/icons/GitHubIcon.svelte'

  export let loading = false
  export let onSignIn: () => void
</script>

<button
  type="button"
  class="btn btn-neutral btn-lg w-full"
  disabled={loading}
  aria-busy={loading}
  on:click={onSignIn}
>
  {#if loading}
    <span class="loading loading-spinner loading-sm"></span>
    Connecting…
  {:else}
    <GitHubIcon />
    Continue with GitHub
  {/if}
</button>
