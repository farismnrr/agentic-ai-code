<script lang="ts">
  import GitHubIcon from '../../../shared/components/icons/GitHubIcon.svelte'
  import RelayIcon from '../../../shared/components/icons/RelayIcon.svelte'
  import type { ConnectedApp } from '../model/connected-app'
  import type { RelayConnectionState } from '../model/relay-connection-state'

  export let app: ConnectedApp
  export let busy = false
  export let onBack: () => void
  export let onEdit: (app: ConnectedApp) => void
  export let onToggle: (app: ConnectedApp) => void
  export let onRemove: (app: ConnectedApp) => void
  export let relayConnection: RelayConnectionState

  let confirmOpen = false
  $: isGitHub = app.clientId.toLowerCase().includes('github')
  $: isRelay = app.clientId === 'relay-agent'

  function copyCallback() {
    void navigator.clipboard?.writeText(app.callbackUrl)
  }

  function connect() {
    const url = new URL(app.callbackUrl)
    url.pathname = '/connections/start'
    url.search = ''
    url.hash = ''
    window.location.assign(url.toString())
  }
</script>

<div>
  <button type="button" class="mb-6 inline-flex items-center gap-2 text-sm text-slate-500 hover:text-slate-900" on:click={onBack}>
    <svg aria-hidden="true" viewBox="0 0 24 24" class="size-4 fill-none stroke-current stroke-2"><path d="m15 18-6-6 6-6"></path></svg>
    Connected Apps
  </button>

  <div class="mb-7 flex flex-wrap items-center justify-between gap-5">
    <div class="flex min-w-0 items-center gap-4">
      <div class="grid size-[88px] place-items-center rounded-xl bg-slate-50 text-slate-900">
        {#if isGitHub}
          <GitHubIcon className="size-12 fill-current" />
        {:else}
          <span class="text-blue-600"><RelayIcon className="size-11" /></span>
        {/if}
      </div>
      <div class="min-w-0">
        <h1 class="truncate text-3xl font-semibold tracking-tight text-slate-950">{app.name}</h1>
        <div class="mt-2 flex flex-wrap items-center gap-5 text-sm text-slate-500">
          <span>{app.clientId}</span>
          <span class="inline-flex items-center gap-2">
            <span class="size-2.5 rounded-full {app.enabled ? 'bg-blue-500' : 'bg-slate-300'}"></span>
            Registry {app.enabled ? 'Enabled' : 'Disabled'}
          </span>
          {#if isRelay}
            <span class="inline-flex items-center gap-2 font-medium {relayConnection.status === 'connected' ? 'text-emerald-700' : relayConnection.status === 'failed' ? 'text-red-600' : 'text-slate-500'}">
              <span class="size-2.5 rounded-full {relayConnection.status === 'connected' ? 'bg-emerald-500' : relayConnection.status === 'failed' ? 'bg-red-500' : 'bg-slate-300'}"></span>
              {relayConnection.status === 'connected' ? 'Connected' : relayConnection.status === 'failed' ? 'Connection failed' : 'Not connected'}
            </span>
          {/if}
        </div>
      </div>
    </div>

    <div class="flex gap-3">
      {#if isRelay}
        <button
          type="button"
          class="btn h-11 min-h-0 border-blue-600 bg-blue-600 px-7 text-white shadow-none hover:border-blue-700 hover:bg-blue-700"
          disabled={!app.enabled || busy}
          on:click={connect}
        >
          {relayConnection.status === 'connected' ? 'Reconnect' : 'Connect'}
        </button>
      {/if}
      <button type="button" class="btn h-11 min-h-0 border-slate-200 bg-white px-7 text-slate-700 shadow-none hover:bg-slate-50" on:click={() => onEdit(app)}>
        Edit
      </button>
      <button
        type="button"
        class="btn h-11 min-h-0 px-7 shadow-none {app.enabled ? 'border-red-200 bg-red-50 text-red-600 hover:bg-red-100' : 'border-blue-200 bg-blue-50 text-blue-700 hover:bg-blue-100'}"
        disabled={busy}
        on:click={() => onToggle(app)}
      >
        {app.enabled ? 'Disable' : 'Enable'}
      </button>
    </div>
  </div>

  <section class="rounded-xl border border-slate-200 bg-white px-6">
    <h2 class="py-5 text-lg font-semibold text-slate-950">App information</h2>
    <dl class="divide-y divide-slate-200">
      <div class="grid gap-2 py-4 sm:grid-cols-[250px_1fr]"><dt class="text-sm text-slate-500">Display name</dt><dd class="text-sm font-medium text-slate-900">{app.name}</dd></div>
      <div class="grid gap-2 py-4 sm:grid-cols-[250px_1fr]"><dt class="text-sm text-slate-500">Client ID</dt><dd class="text-sm font-medium text-slate-900">{app.clientId}</dd></div>
      <div class="grid gap-2 py-4 sm:grid-cols-[250px_1fr]"><dt class="text-sm text-slate-500">Description</dt><dd class="text-sm text-slate-900">{app.description || '—'}</dd></div>
    </dl>
  </section>

  <section class="mt-6 rounded-xl border border-slate-200 bg-white px-6">
    <h2 class="py-5 text-lg font-semibold text-slate-950">Connection settings</h2>
    <dl class="divide-y divide-slate-200">
      <div class="grid items-center gap-2 py-4 sm:grid-cols-[250px_1fr]">
        <dt class="text-sm text-slate-500">Trusted callback URL</dt>
        <dd class="flex min-w-0 items-center gap-2 rounded-lg border border-slate-200 bg-slate-50 px-4 py-3 font-mono text-sm text-slate-700">
          <span class="min-w-0 flex-1 truncate">{app.callbackUrl}</span>
          <button type="button" aria-label="Copy callback URL" class="text-slate-500 hover:text-slate-900" on:click={copyCallback}>
            <svg aria-hidden="true" viewBox="0 0 24 24" class="size-4 fill-none stroke-current stroke-2"><rect x="9" y="9" width="11" height="11" rx="2"></rect><path d="M15 9V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h3"></path></svg>
          </button>
        </dd>
      </div>
      <div class="grid gap-2 py-4 sm:grid-cols-[250px_1fr]"><dt class="text-sm text-slate-500">Assertion lifetime</dt><dd class="text-sm font-medium text-slate-900">{app.assertionTtlSeconds} seconds</dd></div>
    </dl>
  </section>

  <section class="mt-6 rounded-xl border border-slate-200 bg-white px-6">
    <h2 class="py-5 text-lg font-semibold text-slate-950">Activity</h2>
    <dl class="divide-y divide-slate-200">
      <div class="grid gap-2 py-4 sm:grid-cols-[250px_1fr]"><dt class="text-sm text-slate-500">Registry status</dt><dd class="text-sm font-medium text-slate-900">{app.enabled ? 'Enabled' : 'Disabled'}</dd></div>
      {#if isRelay}
        <div class="grid gap-2 py-4 sm:grid-cols-[250px_1fr]">
          <dt class="text-sm text-slate-500">Relay connection</dt>
          <dd class="text-sm font-medium {relayConnection.status === 'connected' ? 'text-emerald-700' : relayConnection.status === 'failed' ? 'text-red-600' : 'text-slate-500'}">
            {relayConnection.status === 'connected' ? 'Connected' : relayConnection.status === 'failed' ? 'Connection failed' : 'Not connected'}
          </dd>
        </div>
        {#if relayConnection.status === 'connected'}
          <div class="grid gap-2 py-4 sm:grid-cols-[250px_1fr]"><dt class="text-sm text-slate-500">Connection ID</dt><dd class="break-all font-mono text-sm text-slate-700">{relayConnection.connectionId}</dd></div>
        {/if}
      {/if}
      <div class="grid gap-2 py-4 sm:grid-cols-[250px_1fr]"><dt class="text-sm text-slate-500">Usage history</dt><dd class="text-sm text-slate-500">Not tracked by SSO</dd></div>
    </dl>
  </section>

  <div class="mt-7 border-t border-slate-200 pt-4">
    <button type="button" class="text-sm font-medium text-red-600 hover:text-red-700" on:click={() => { confirmOpen = true }}>
      Remove app
    </button>
  </div>

  {#if confirmOpen}
    <div class="fixed inset-0 z-50 grid place-items-center p-4">
      <button type="button" aria-label="Close confirmation" class="absolute inset-0 bg-slate-950/40" on:click={() => { confirmOpen = false }}></button>
      <div role="dialog" aria-modal="true" class="relative z-10 w-full max-w-md rounded-xl bg-white p-6 shadow-2xl">
        <h2 class="text-lg font-semibold text-slate-950">Remove {app.name}?</h2>
        <p class="mt-2 text-sm leading-6 text-slate-500">This removes the app registration from SSO. It does not represent an active Relay connection.</p>
        <div class="mt-6 flex justify-end gap-3">
          <button type="button" class="btn h-10 min-h-0 border-slate-200 bg-white text-slate-700 shadow-none" on:click={() => { confirmOpen = false }}>Cancel</button>
          <button type="button" class="btn h-10 min-h-0 border-red-600 bg-red-600 text-white shadow-none hover:bg-red-700" disabled={busy} on:click={() => onRemove(app)}>Remove</button>
        </div>
      </div>
    </div>
  {/if}
</div>
