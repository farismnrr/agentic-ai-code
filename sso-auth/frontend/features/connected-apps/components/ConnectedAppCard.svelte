<script lang="ts">
  import GitHubIcon from '../../../shared/components/icons/GitHubIcon.svelte'
  import RelayIcon from '../../../shared/components/icons/RelayIcon.svelte'
  import type { ConnectedApp } from '../model/connected-app'

  export let app: ConnectedApp
  export let onOpen: (app: ConnectedApp) => void

  $: isGitHub = app.clientId.toLowerCase().includes('github')
</script>

<button
  type="button"
  class="grid w-full grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-4 px-5 py-5 text-left hover:bg-slate-50 sm:grid-cols-[auto_minmax(0,1fr)_150px_auto]"
  on:click={() => onOpen(app)}
>
  <span class="grid size-14 place-items-center rounded-xl bg-slate-50 text-slate-900">
    {#if isGitHub}
      <GitHubIcon className="size-9 fill-current" />
    {:else}
      <span class="text-blue-600"><RelayIcon className="size-8" /></span>
    {/if}
  </span>

  <span class="min-w-0">
    <span class="block truncate text-base font-semibold text-slate-950">{app.name}</span>
    <span class="mt-0.5 block truncate text-sm text-slate-500">{app.clientId}</span>
  </span>

  <span class="hidden items-center gap-2 text-sm sm:flex">
    <span class="size-2.5 rounded-full {app.enabled ? 'bg-emerald-400' : 'bg-slate-300'}"></span>
    <span class="text-slate-600">{app.enabled ? 'Connected' : 'Disabled'}</span>
  </span>

  <svg aria-hidden="true" viewBox="0 0 24 24" class="size-5 fill-none stroke-slate-500 stroke-2">
    <path d="m9 5 7 7-7 7"></path>
  </svg>
</button>
