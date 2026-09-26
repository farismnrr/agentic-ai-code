<script lang="ts">
  import type { ConnectedApp } from '../model/connected-app'

  export let app: ConnectedApp
  export let saving = false
  export let clientIdLocked = false
  export let onSave: (app: ConnectedApp) => void
</script>

<form class="space-y-4" on:submit|preventDefault={() => onSave(app)}>
  <label class="form-control block">
    <span class="mb-1.5 block text-xs font-semibold text-slate-600">Client ID / audience</span>
    <input
      class="input input-bordered w-full bg-white"
      bind:value={app.clientId}
      disabled={clientIdLocked}
      minlength="3"
      maxlength="64"
      required
    />
  </label>

  <label class="form-control block">
    <span class="mb-1.5 block text-xs font-semibold text-slate-600">Display name</span>
    <input
      class="input input-bordered w-full bg-white"
      bind:value={app.name}
      maxlength="80"
      required
    />
  </label>

  <label class="form-control block">
    <span class="mb-1.5 block text-xs font-semibold text-slate-600">Description</span>
    <textarea
      class="textarea textarea-bordered min-h-20 w-full bg-white"
      bind:value={app.description}
      maxlength="240"
    ></textarea>
  </label>

  <label class="form-control block">
    <span class="mb-1.5 block text-xs font-semibold text-slate-600">Trusted callback URL</span>
    <input
      type="url"
      class="input input-bordered w-full bg-white"
      bind:value={app.callbackUrl}
      placeholder="https://relay.example.com/connections/callback"
      required
    />
    <span class="mt-1 block text-xs leading-5 text-slate-400">
      Requests cannot override this URL during connect.
    </span>
  </label>

  <label class="form-control block">
    <span class="mb-1.5 block text-xs font-semibold text-slate-600">Assertion TTL</span>
    <div class="join w-full">
      <input
        type="number"
        class="input input-bordered join-item w-full bg-white"
        bind:value={app.assertionTtlSeconds}
        min="30"
        max="300"
        required
      />
      <span class="join-item flex items-center border border-slate-300 bg-slate-50 px-4 text-sm text-slate-500">
        seconds
      </span>
    </div>
  </label>

  <label class="flex cursor-pointer items-center justify-between rounded-xl border border-slate-200 px-4 py-3">
    <div>
      <span class="block text-sm font-medium text-slate-800">Connection enabled</span>
      <span class="mt-0.5 block text-xs text-slate-500">Disabled apps cannot start or finish a handoff.</span>
    </div>
    <input type="checkbox" class="toggle toggle-primary" bind:checked={app.enabled} />
  </label>

  <button type="submit" class="btn btn-primary w-full" disabled={saving}>
    {saving ? 'Saving…' : 'Save connected app'}
  </button>
</form>
