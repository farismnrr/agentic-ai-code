<script lang="ts">
  export let onSignOut: () => void
</script>

<button type="button" class="btn btn-outline w-full" on:click={onSignOut}>
  Sign out
</button>
