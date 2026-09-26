<script lang="ts">
  import type { ConnectedApp } from '../model/connected-app'

  export let app: ConnectedApp
  export let onEdit: (app: ConnectedApp) => void
</script>

<article class="rounded-xl border border-slate-200 bg-slate-50/70 p-4">
  <div class="flex items-start justify-between gap-4">
    <div class="min-w-0">
      <div class="flex flex-wrap items-center gap-2">
        <h3 class="truncate font-semibold text-slate-900">{app.name}</h3>
        <span class:badge-success={app.enabled} class:badge-ghost={!app.enabled} class="badge badge-sm">
          {app.enabled ? 'Enabled' : 'Disabled'}
        </span>
      </div>
      <p class="mt-1 font-mono text-xs text-slate-500">{app.clientId}</p>
    </div>

    <button
      type="button"
      class="btn btn-ghost btn-sm"
      on:click={() => onEdit(app)}
    >
      Edit
    </button>
  </div>

  {#if app.description}
    <p class="mt-3 text-sm leading-5 text-slate-600">{app.description}</p>
  {/if}

  <dl class="mt-4 space-y-2 text-xs">
    <div>
      <dt class="font-medium text-slate-500">Trusted callback</dt>
      <dd class="mt-0.5 break-all text-slate-700">{app.callbackUrl}</dd>
    </div>
    <div class="flex items-center justify-between gap-4">
      <dt class="font-medium text-slate-500">Assertion TTL</dt>
      <dd class="text-slate-700">{app.assertionTtlSeconds}s</dd>
    </div>
  </dl>
</article>
