<script lang="ts">
  import type { ConnectedApp } from '../model/connected-app'

  export let app: ConnectedApp
  export let saving = false
  export let clientIdLocked = false
  export let submitLabel = 'Add app'
  export let onSave: (app: ConnectedApp) => void
  export let onCancel: () => void

  const lifetimes = [30, 60, 90, 120, 180, 300]
</script>

<form class="space-y-4" on:submit|preventDefault={() => onSave(app)}>
  <label class="block">
    <span class="mb-1.5 block text-sm font-semibold text-slate-900">App name</span>
    <input
      class="input input-bordered h-10 w-full border-slate-300 bg-white text-sm focus:border-blue-500 focus:outline-none"
      bind:value={app.name}
      maxlength="80"
      required
    />
    <span class="mt-1 block text-xs text-slate-500">Shown to users when they review this connection.</span>
  </label>

  <label class="block">
    <span class="mb-1.5 block text-sm font-semibold text-slate-900">Client ID</span>
    <input
      class="input input-bordered h-10 w-full border-slate-300 bg-white text-sm focus:border-blue-500 focus:outline-none"
      bind:value={app.clientId}
      disabled={clientIdLocked}
      minlength="3"
      maxlength="64"
      required
    />
    <span class="mt-1 block text-xs text-slate-500">A unique identifier for this app.</span>
  </label>

  <label class="block">
    <span class="mb-1.5 block text-sm font-semibold text-slate-900">Description</span>
    <textarea
      class="textarea textarea-bordered min-h-20 w-full border-slate-300 bg-white text-sm focus:border-blue-500 focus:outline-none"
      bind:value={app.description}
      maxlength="240"
    ></textarea>
  </label>

  <label class="block">
    <span class="mb-1.5 block text-sm font-semibold text-slate-900">Trusted callback URL</span>
    <input
      type="url"
      class="input input-bordered h-10 w-full border-slate-300 bg-white text-sm focus:border-blue-500 focus:outline-none"
      bind:value={app.callbackUrl}
      placeholder="https://relay.example.com/connections/callback"
      required
    />
    <span class="mt-1 block text-xs text-slate-500">After authentication, Masih Awam will only return users to this address.</span>
  </label>

  <label class="block">
    <span class="mb-1.5 block text-sm font-semibold text-slate-900">Assertion lifetime</span>
    <select
      class="select select-bordered h-10 w-full border-slate-300 bg-white text-sm focus:border-blue-500 focus:outline-none"
      bind:value={app.assertionTtlSeconds}
    >
      {#each lifetimes as seconds}
        <option value={seconds}>{seconds} seconds</option>
      {/each}
    </select>
  </label>

  <label class="flex items-center gap-4 py-1">
    <span class="text-sm font-semibold text-slate-900">Enabled</span>
    <input
      type="checkbox"
      class="toggle toggle-sm border-slate-300 bg-slate-200 checked:border-blue-600 checked:bg-blue-600"
      bind:checked={app.enabled}
    />
  </label>

  <div class="flex items-start gap-2 border-t border-slate-200 pt-4 text-xs text-slate-500">
    <svg aria-hidden="true" viewBox="0 0 24 24" class="mt-0.5 size-4 shrink-0 fill-none stroke-current stroke-2">
      <circle cx="12" cy="12" r="9"></circle><path d="M12 11v5m0-8h.01"></path>
    </svg>
    <p>Signing secrets are managed securely by the server.</p>
  </div>

  <div class="flex justify-end gap-3 pt-1">
    <button type="button" class="btn h-10 min-h-0 border-slate-200 bg-white px-5 text-slate-600 shadow-none hover:bg-slate-50" on:click={onCancel}>
      Cancel
    </button>
    <button type="submit" class="btn h-10 min-h-0 border-blue-600 bg-blue-600 px-5 text-white shadow-none hover:border-blue-700 hover:bg-blue-700" disabled={saving}>
      {saving ? 'Saving…' : submitLabel}
    </button>
  </div>
</form>
