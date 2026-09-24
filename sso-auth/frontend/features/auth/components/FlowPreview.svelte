<script lang="ts">
  import type { AuthState } from '../model/auth-state'

  export let onPreview: (state: AuthState) => void

  const demoUser = {
    login: 'farismnrr',
    avatarUrl: 'https://github.com/identicons/farismnrr.png'
  }

  const buttonClass = 'btn btn-sm border-slate-200 bg-white font-medium text-slate-600 shadow-none hover:border-blue-300 hover:bg-blue-50 hover:text-blue-700'
</script>

<section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
  <div class="flex items-start gap-3">
    <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-slate-100 text-slate-600">
      <svg aria-hidden="true" viewBox="0 0 24 24" class="size-5 fill-none stroke-current stroke-2">
        <path d="M4 7h10M18 7h2M4 17h2M10 17h10M4 12h4M12 12h8"></path>
        <circle cx="16" cy="7" r="2"></circle>
        <circle cx="8" cy="17" r="2"></circle>
        <circle cx="10" cy="12" r="2"></circle>
      </svg>
    </div>
    <div>
      <h2 class="font-semibold text-slate-900">Flow preview</h2>
      <p class="mt-0.5 text-xs text-slate-500">Development utility</p>
    </div>
  </div>

  <p class="mt-4 text-sm leading-6 text-slate-500">
    Quickly switch between authentication states to preview the experience.
  </p>

  <div class="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-3">
    <button class={buttonClass} type="button" on:click={() => onPreview({ status: 'signed_out' })}>Signed out</button>
    <button class={buttonClass} type="button" on:click={() => onPreview({ status: 'loading' })}>Loading</button>
    <button class={buttonClass} type="button" on:click={() => onPreview({ status: 'callback_processing' })}>Callback</button>
    <button class={buttonClass} type="button" on:click={() => onPreview({ status: 'authenticated', user: demoUser })}>Authenticated</button>
    <button class={buttonClass} type="button" on:click={() => onPreview({ status: 'session_expired' })}>Expired</button>
    <button class={buttonClass} type="button" on:click={() => onPreview({ status: 'error', message: 'GitHub authentication could not be completed.' })}>Error</button>
  </div>
</section>
