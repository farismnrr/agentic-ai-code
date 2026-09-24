<script lang="ts">
  import type { AuthState } from '../model/auth-state'

  export let onPreview: (state: AuthState) => void

  const demoUser = {
    login: 'farismnrr',
    avatarUrl: 'https://github.com/identicons/farismnrr.png'
  }
</script>

<div class="collapse collapse-arrow border border-base-300 bg-base-100">
  <input type="checkbox" />
  <div class="collapse-title text-sm font-medium text-base-content/75">
    Preview UI states
  </div>

  <div class="collapse-content">
    <div class="flex flex-wrap gap-2 pt-1">
      <button class="btn btn-xs" type="button" on:click={() => onPreview({ status: 'signed_out' })}>
        Login
      </button>
      <button class="btn btn-xs" type="button" on:click={() => onPreview({ status: 'loading' })}>
        Loading
      </button>
      <button class="btn btn-xs" type="button" on:click={() => onPreview({ status: 'callback_processing' })}>
        Callback
      </button>
      <button
        class="btn btn-xs"
        type="button"
        on:click={() => onPreview({ status: 'authenticated', user: demoUser })}
      >
        Authenticated
      </button>
      <button class="btn btn-xs" type="button" on:click={() => onPreview({ status: 'session_expired' })}>
        Expired
      </button>
      <button
        class="btn btn-xs"
        type="button"
        on:click={() => onPreview({ status: 'error', message: 'GitHub authentication could not be completed.' })}
      >
        Error
      </button>
    </div>
  </div>
</div>
