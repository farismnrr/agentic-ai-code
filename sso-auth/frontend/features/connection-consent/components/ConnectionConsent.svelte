<script lang="ts">
  import { onMount } from 'svelte'

  import RelayIcon from '../../../shared/components/icons/RelayIcon.svelte'
  import { loadConnectionApp, type ConnectionAppInfo } from '../api/connection-consent-api'

  let app: ConnectionAppInfo | null = null
  let error = ''
  let loading = true

  const params = new URL(window.location.href).searchParams
  const clientId = params.get('client_id') ?? ''
  const connectionState = params.get('connection_state') ?? ''

  onMount(() => { void load() })

  async function load() {
    if (!clientId || !connectionState) {
      error = 'This connection request is incomplete.'
      loading = false
      return
    }

    try {
      app = await loadConnectionApp(clientId)
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'This connected app is unavailable.'
    } finally {
      loading = false
    }
  }

  function connect() {
    if (!app) return
    const url = new URL('/auth/github', window.location.origin)
    url.searchParams.set('client_id', clientId)
    url.searchParams.set('connection_state', connectionState)
    window.location.assign(url.toString())
  }

  function cancel() {
    window.location.assign('/')
  }
</script>

<div class="min-h-screen bg-white">
  <header class="flex h-20 items-center border-b border-slate-200 px-6 sm:px-10">
    <img src="/assets/masihawam-logo.webp" alt="Masih Awam" class="h-9 w-auto object-contain" />
  </header>

  <div class="flex min-h-[calc(100vh-5rem)] items-center justify-center px-4 py-10">
    <section class="w-full max-w-[700px] rounded-xl border border-slate-200 bg-white px-7 py-8 sm:px-11 sm:py-10">
      {#if loading}
        <div class="space-y-5">
          <div class="skeleton mx-auto size-24 rounded-2xl"></div>
          <div class="skeleton mx-auto h-8 w-72"></div>
          <div class="skeleton mx-auto h-5 w-96 max-w-full"></div>
        </div>
      {:else if error}
        <div class="py-6 text-center">
          <div class="mx-auto grid size-16 place-items-center rounded-2xl bg-red-50 text-red-600">
            <svg aria-hidden="true" viewBox="0 0 24 24" class="size-7 fill-none stroke-current stroke-2"><circle cx="12" cy="12" r="9"></circle><path d="M12 7v6m0 4h.01"></path></svg>
          </div>
          <h1 class="mt-5 text-2xl font-semibold text-slate-950">Unable to connect this app</h1>
          <p class="mt-2 text-sm text-slate-500">{error}</p>
          <button type="button" class="btn mt-7 border-slate-200 bg-white text-slate-700 shadow-none" on:click={cancel}>Back to Masih Awam</button>
        </div>
      {:else if app}
        <div class="text-center">
          <div class="mx-auto grid size-24 place-items-center rounded-2xl bg-blue-50 text-blue-600">
            <RelayIcon className="size-13" />
          </div>
          <h1 class="mt-5 text-3xl font-semibold tracking-tight text-slate-950">Connect {app.name}</h1>
          <p class="mt-2 text-base text-slate-500">{app.name} wants to connect to your Masih Awam account.</p>
        </div>

        <div class="my-8 border-t border-slate-200"></div>

        <h2 class="text-base font-semibold text-slate-950">This connection will allow the app to:</h2>

        <div class="mt-6 space-y-5">
          <div class="flex items-center gap-5">
            <span class="grid size-16 shrink-0 place-items-center rounded-xl bg-slate-50 text-slate-500">
              <svg aria-hidden="true" viewBox="0 0 24 24" class="size-7 fill-none stroke-current stroke-[1.8]"><circle cx="12" cy="7" r="3"></circle><path d="M5 21v-2a7 7 0 0 1 14 0v2"></path></svg>
            </span>
            <div>
              <p class="font-semibold text-slate-950">Identify your Masih Awam account</p>
              <p class="mt-1 text-sm text-slate-500">Basic profile information</p>
            </div>
          </div>

          <div class="flex items-center gap-5">
            <span class="grid size-16 shrink-0 place-items-center rounded-xl bg-slate-50 text-slate-500">
              <svg aria-hidden="true" viewBox="0 0 24 24" class="size-7 fill-none stroke-current stroke-[1.8]"><path d="m12 3 7 3v5c0 4.5-2.8 7.7-7 10-4.2-2.3-7-5.5-7-10V6l7-3Z"></path><path d="m9.5 12 1.8 1.8 3.7-4"></path></svg>
            </span>
            <div>
              <p class="font-semibold text-slate-950">Receive a secure connection handoff</p>
              <p class="mt-1 text-sm text-slate-500">Used to establish the Relay connection</p>
            </div>
          </div>
        </div>

        <div class="mt-7 flex items-start gap-3 border-t border-slate-200 pt-6 text-sm text-slate-500">
          <svg aria-hidden="true" viewBox="0 0 24 24" class="mt-0.5 size-5 shrink-0 fill-none stroke-current stroke-[1.8]"><rect x="5" y="10" width="14" height="10" rx="2"></rect><path d="M8 10V7a4 4 0 0 1 8 0v3"></path></svg>
          <p>You can disconnect this app anytime from Connected Apps.</p>
        </div>

        <div class="mt-8 flex justify-end gap-3">
          <button type="button" class="btn h-12 min-h-0 border-slate-200 bg-slate-50 px-8 text-slate-600 shadow-none hover:bg-slate-100" on:click={cancel}>Cancel</button>
          <button type="button" class="btn h-12 min-h-0 border-blue-600 bg-blue-600 px-8 text-white shadow-none hover:border-blue-700 hover:bg-blue-700" on:click={connect}>Connect</button>
        </div>
      {/if}
    </section>
  </div>
</div>
