<script lang="ts">
  import { ConnectedAppsPanel } from '../../connected-apps'
  import type { AuthenticatedSession } from '../model/auth-state'
  import DashboardSidebar from './DashboardSidebar.svelte'
  import LogoutButton from './LogoutButton.svelte'

  export let session: AuthenticatedSession
  export let onSignOut: () => void
</script>

<div class="min-h-screen bg-white">
  <header class="sticky top-0 z-30 flex h-20 border-b border-slate-200 bg-white">
    <div class="flex min-w-0 flex-1 items-center px-5 md:w-[298px] md:flex-none md:border-r md:border-slate-200 md:px-8">
      <img src="/assets/masihawam-logo.webp" alt="Masih Awam" class="h-9 w-auto object-contain" />
    </div>

    <div class="ml-auto flex items-center justify-end px-4 sm:px-7">
      <details class="group relative">
        <summary class="flex cursor-pointer list-none items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-slate-50 [&::-webkit-details-marker]:hidden">
          {#if session.user.avatarUrl}
            <img
              src={session.user.avatarUrl}
              alt=""
              class="size-11 rounded-full border border-slate-200 object-cover"
            />
          {:else}
            <div class="grid size-11 place-items-center rounded-full bg-slate-100 text-sm font-semibold text-slate-600">
              {session.user.login.slice(0, 2).toUpperCase()}
            </div>
          {/if}
          <div class="hidden min-w-0 text-left sm:block">
            <p class="max-w-48 truncate text-sm font-semibold text-slate-950">@{session.user.login}</p>
            <p class="text-xs text-slate-500">GitHub Connected</p>
          </div>
          <svg aria-hidden="true" viewBox="0 0 24 24" class="hidden size-4 fill-none stroke-slate-500 stroke-2 sm:block">
            <path d="m7 10 5 5 5-5"></path>
          </svg>
        </summary>

        <div class="absolute right-0 mt-2 w-56 rounded-xl border border-slate-200 bg-white p-2 shadow-xl shadow-slate-900/10">
          <p class="px-2 py-2 text-xs text-slate-500">Signed in with GitHub</p>
          <LogoutButton {onSignOut} />
        </div>
      </details>
    </div>
  </header>

  <div class="flex min-h-[calc(100vh-5rem)]">
    <DashboardSidebar />
    <main class="min-w-0 flex-1 bg-white px-5 py-8 sm:px-8 lg:px-10 lg:py-9">
      <div class="mx-auto w-full max-w-6xl">
        <ConnectedAppsPanel />
      </div>
    </main>
  </div>
</div>
