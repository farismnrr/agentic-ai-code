<script lang="ts">
  import GitHubIcon from '../../../shared/components/icons/GitHubIcon.svelte'

  export let loading = false
</script>

<a
  class:btn-disabled={loading}
  class="btn btn-neutral btn-lg w-full"
  href={loading ? undefined : '/auth/github'}
  aria-disabled={loading}
>
  {#if loading}
    <span class="loading loading-spinner loading-sm"></span>
    Connecting...
  {:else}
    <GitHubIcon />
    Continue with GitHub
  {/if}
</a>
