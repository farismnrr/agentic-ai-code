<script lang="ts">
  import { onMount } from 'svelte'
  import { Agentation } from 'sv-agentation'

  let enabled = false

  onMount(() => {
    const params = new URLSearchParams(window.location.search)
    enabled = params.get('inspect') === '1'
  })
</script>

{#if enabled}
  <Agentation
    outputMode="detailed"
    includeComponentContext={true}
    includeComputedStyles={true}
    toolbarPosition="bottom-right"
  />
{/if}
