<script lang="ts">
  import GitHubIcon from '../../../shared/components/icons/GitHubIcon.svelte'

  export let loading = false
  export let label = 'Continue with GitHub'
  export let onSignIn: () => void
</script>

<button
  type="button"
  class="btn h-12 w-full border-slate-900 bg-slate-900 text-white shadow-sm hover:border-slate-800 hover:bg-slate-800"
  disabled={loading}
  aria-busy={loading}
  on:click={onSignIn}
>
  {#if loading}
    <span class="loading loading-spinner loading-sm"></span>
    Connecting…
  {:else}
    <GitHubIcon />
    {label}
  {/if}
</button>
