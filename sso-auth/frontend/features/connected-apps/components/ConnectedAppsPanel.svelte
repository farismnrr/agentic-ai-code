<script lang="ts">
  import { onMount } from 'svelte'

  import { deleteConnectedApp, listConnectedApps, saveConnectedApp } from '../api/connected-apps-api'
  import { emptyConnectedApp, type ConnectedApp } from '../model/connected-app'
  import ConnectedAppCard from './ConnectedAppCard.svelte'
  import ConnectedAppDetails from './ConnectedAppDetails.svelte'
  import ConnectedAppDialog from './ConnectedAppDialog.svelte'

  let apps: ConnectedApp[] = []
  let selected: ConnectedApp | null = null
  let draft = emptyConnectedApp()
  let dialogMode: 'create' | 'edit' | null = null
  let loading = true
  let saving = false
  let busy = false
  let message = ''
  let error = ''

  onMount(() => {
    applyConnectionResult()
    void load()
  })

  function applyConnectionResult() {
    const url = new URL(window.location.href)
    if (url.searchParams.get('relay_connection') !== 'connected') return

    message = 'Relay connected successfully.'
    url.searchParams.delete('relay_connection')
    url.searchParams.delete('connection_id')
    window.history.replaceState({}, '', `${url.pathname}${url.search}${url.hash}`)
  }

  async function load() {
    loading = true
    error = ''
    try {
      apps = await listConnectedApps()
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Unable to load connected apps.'
    } finally {
      loading = false
    }
  }

  function createNew() {
    draft = emptyConnectedApp()
    dialogMode = 'create'
    message = ''
    error = ''
  }

  function edit(app: ConnectedApp) {
    draft = { ...app }
    dialogMode = 'edit'
    message = ''
    error = ''
  }

  async function save(app: ConnectedApp) {
    saving = true
    error = ''
    try {
      const saved = await saveConnectedApp({ ...app })
      const exists = apps.some(item => item.clientId === saved.clientId)
      apps = exists
        ? apps.map(item => item.clientId === saved.clientId ? saved : item)
        : [...apps, saved]
      if (selected?.clientId === saved.clientId) selected = saved
      dialogMode = null
      message = exists ? 'App registration updated.' : 'App registration added.'
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Unable to save connected app.'
    } finally {
      saving = false
    }
  }

  async function toggle(app: ConnectedApp) {
    busy = true
    error = ''
    try {
      const saved = await saveConnectedApp({ ...app, enabled: !app.enabled })
      apps = apps.map(item => item.clientId === saved.clientId ? saved : item)
      selected = saved
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Unable to update connected app.'
    } finally {
      busy = false
    }
  }

  async function remove(app: ConnectedApp) {
    busy = true
    error = ''
    try {
      await deleteConnectedApp(app.clientId)
      apps = apps.filter(item => item.clientId !== app.clientId)
      selected = null
      message = 'App registration removed.'
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Unable to remove app registration.'
    } finally {
      busy = false
    }
  }
</script>

{#if error && !dialogMode}
  <div class="alert mb-5 border-red-200 bg-red-50 text-sm text-red-700">{error}</div>
{:else if message}
  <div class="alert mb-5 border-emerald-200 bg-emerald-50 text-sm text-emerald-700">{message}</div>
{/if}

{#if selected}
  <ConnectedAppDetails
    app={selected}
    {busy}
    onBack={() => { selected = null }}
    onEdit={edit}
    onToggle={toggle}
    onRemove={remove}
  />
{:else}
  <div class="flex flex-wrap items-start justify-between gap-5">
    <header>
      <p class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-500">Connected Apps</p>
      <h1 class="mt-2 text-4xl font-semibold tracking-tight text-slate-950">Connected Apps</h1>
      <p class="mt-2 text-lg text-slate-500">Apps and services registered to connect to your Masih Awam account.</p>
    </header>
    <button type="button" class="btn h-12 min-h-0 border-blue-600 bg-blue-600 px-7 text-white shadow-none hover:border-blue-700 hover:bg-blue-700" on:click={createNew}>
      Add app
    </button>
  </div>

  <section class="mt-9 overflow-hidden rounded-xl border border-slate-200 bg-white">
    {#if loading}
      <div class="space-y-4 p-7">
        <div class="skeleton h-16 w-full"></div>
        <div class="skeleton h-16 w-full"></div>
      </div>
    {:else}
      <div class="divide-y divide-slate-200">
        {#each apps as app (app.clientId)}
          <ConnectedAppCard {app} onOpen={(item) => { selected = item }} />
        {/each}

        <button type="button" class="flex w-full items-center gap-5 px-8 py-6 text-left hover:bg-slate-50" on:click={createNew}>
          <span class="grid size-14 place-items-center rounded-xl bg-slate-50 text-slate-500">
            <svg aria-hidden="true" viewBox="0 0 24 24" class="size-7 fill-none stroke-current stroke-[1.8]"><path d="M12 5v14M5 12h14"></path></svg>
          </span>
          <span>
            <span class="block text-base font-semibold text-slate-950">Add another app</span>
            <span class="mt-0.5 block text-sm text-slate-500">Register a new app</span>
          </span>
          <svg aria-hidden="true" viewBox="0 0 24 24" class="ml-auto size-5 fill-none stroke-slate-500 stroke-2"><path d="m9 5 7 7-7 7"></path></svg>
        </button>
      </div>
    {/if}
  </section>

  <p class="mt-7 text-sm text-slate-500">Registered apps can only return users to callback URLs you configure here.</p>
{/if}

{#if dialogMode}
  <ConnectedAppDialog
    app={draft}
    {saving}
    {error}
    mode={dialogMode}
    onSave={save}
    onClose={() => { dialogMode = null; error = '' }}
  />
{/if}
