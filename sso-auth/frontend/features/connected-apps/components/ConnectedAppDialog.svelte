<script lang="ts">
  import type { ConnectedApp } from '../model/connected-app'
  import ConnectedAppForm from './ConnectedAppForm.svelte'

  export let app: ConnectedApp
  export let saving = false
  export let error = ''
  export let mode: 'create' | 'edit' = 'create'
  export let onSave: (app: ConnectedApp) => void
  export let onClose: () => void
</script>

<div class="fixed inset-0 z-50 grid place-items-center p-4" role="presentation">
  <button type="button" aria-label="Close dialog" class="absolute inset-0 bg-slate-950/40" on:click={onClose}></button>

  <div
    role="dialog"
    aria-modal="true"
    aria-labelledby="connected-app-dialog-title"
    class="relative z-10 w-full max-w-[630px] overflow-hidden rounded-xl border border-slate-200 bg-white shadow-2xl shadow-slate-950/20"
  >
    <div class="max-h-[90vh] overflow-y-auto overscroll-contain p-6 sm:p-8">
      <div class="mb-5 flex items-start justify-between gap-4">
        <div>
          <h2 id="connected-app-dialog-title" class="text-xl font-semibold tracking-tight text-slate-950">
            {mode === 'create' ? 'Add connected app' : 'Edit connected app'}
          </h2>
          <p class="mt-1 text-sm text-slate-500">
            {mode === 'create' ? 'Register an app that can securely connect to Masih Awam.' : 'Update this app’s trusted connection settings.'}
          </p>
        </div>
        <button type="button" aria-label="Close" class="grid size-9 place-items-center rounded-lg text-slate-500 hover:bg-slate-100" on:click={onClose}>
          <svg aria-hidden="true" viewBox="0 0 24 24" class="size-5 fill-none stroke-current stroke-2">
            <path d="m6 6 12 12M18 6 6 18"></path>
          </svg>
        </button>
      </div>

      {#if error}
        <div class="alert mb-4 border-red-200 bg-red-50 text-sm text-red-700">{error}</div>
      {/if}

      <ConnectedAppForm
        {app}
        {saving}
        clientIdLocked={mode === 'edit'}
        submitLabel={mode === 'create' ? 'Add app' : 'Save changes'}
        {onSave}
        onCancel={onClose}
      />
    </div>
  </div>
</div>
