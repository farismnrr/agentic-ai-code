<script lang="ts">
  export let onSignOut: () => void
</script>

<button
  type="button"
  class="btn w-full border-slate-200 bg-white font-medium text-red-600 shadow-none hover:border-red-200 hover:bg-red-50"
  on:click={onSignOut}
>
  Sign out
</button>
