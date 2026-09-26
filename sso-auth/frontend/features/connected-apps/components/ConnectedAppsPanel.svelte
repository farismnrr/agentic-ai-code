<script lang="ts">
  import { onMount } from 'svelte'

  import { listConnectedApps, saveConnectedApp } from '../api/connected-apps-api'
  import { defaultRelayApp, type ConnectedApp } from '../model/connected-app'
  import ConnectedAppCard from './ConnectedAppCard.svelte'
  import ConnectedAppForm from './ConnectedAppForm.svelte'

  let apps: ConnectedApp[] = []
  let draft = defaultRelayApp()
  let editingId: string | null = null
  let loading = true
  let saving = false
  let message = ''
  let error = ''

  onMount(() => {
    void load()
  })

  async function load() {
    loading = true
    error = ''
    try {
      apps = await listConnectedApps()
      if (apps.length > 0) {
        edit(apps[0])
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Unable to load connected apps.'
    } finally {
      loading = false
    }
  }

  function edit(app: ConnectedApp) {
    draft = { ...app }
    editingId = app.clientId
    message = ''
    error = ''
  }

  function createNew() {
    draft = defaultRelayApp()
    editingId = null
    message = ''
    error = ''
  }

  async function save(app: ConnectedApp) {
    saving = true
    message = ''
    error = ''
    try {
      const saved = await saveConnectedApp({ ...app })
      const existing = apps.findIndex((item) => item.clientId === saved.clientId)
      if (existing >= 0) {
        apps = apps.map((item) => item.clientId === saved.clientId ? saved : item)
      } else {
        apps = [...apps, saved]
      }
      edit(saved)
      message = 'Connected app saved.'
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Unable to save connected app.'
    } finally {
      saving = false
    }
  }
</script>

<section class="w-full rounded-2xl border border-slate-200 bg-white shadow-sm">
  <div class="p-6 sm:p-8">
    <div class="flex flex-wrap items-start justify-between gap-4">
      <header class="space-y-2">
        <p class="text-xs font-semibold uppercase tracking-[0.14em] text-blue-600">Connected apps</p>
        <h2 class="text-2xl font-semibold tracking-tight text-slate-950">App registry</h2>
        <p class="max-w-xl text-sm leading-6 text-slate-600">
          Register trusted callback settings here. Secrets stay in server configuration.
        </p>
      </header>

      <button type="button" class="btn btn-ghost btn-sm" on:click={createNew}>
        New app
      </button>
    </div>

    {#if error}
      <div class="alert alert-error mt-5 text-sm">{error}</div>
    {:else if message}
      <div class="alert alert-success mt-5 text-sm">{message}</div>
    {/if}

    <div class="mt-6 grid gap-6 xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
      <div class="space-y-3">
        {#if loading}
          <div class="skeleton h-28 w-full"></div>
        {:else if apps.length === 0}
          <div class="rounded-xl border border-dashed border-slate-300 p-5 text-sm leading-6 text-slate-500">
            No apps registered yet. The form is prefilled for the local Relay service.
          </div>
        {:else}
          {#each apps as app (app.clientId)}
            <ConnectedAppCard {app} onEdit={edit} />
          {/each}
        {/if}
      </div>

      <ConnectedAppForm
        app={draft}
        {saving}
        clientIdLocked={editingId !== null}
        onSave={save}
      />
    </div>
  </div>
</section>
