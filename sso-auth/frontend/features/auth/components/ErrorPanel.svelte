<script lang="ts">
  import AuthPanel from './AuthPanel.svelte'

  export let message: string
  export let onSignIn: () => void
</script>

<AuthPanel
  title="Authentication error"
  message={message || "We couldn't complete GitHub sign-in. Please try again."}
>
  <div class="mb-6 flex justify-center">
    <div class="grid size-14 place-items-center rounded-full border border-red-200 bg-red-50 text-2xl font-semibold text-red-600">
      !
    </div>
  </div>

  <button type="button" class="btn w-full border-red-600 bg-red-600 text-white hover:border-red-700 hover:bg-red-700" on:click={onSignIn}>
    Try again
  </button>

  <button type="button" class="btn btn-ghost mt-2 w-full text-slate-600">
    View details
  </button>
</AuthPanel>
