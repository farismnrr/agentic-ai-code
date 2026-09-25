<script lang="ts">
  import { Agentation } from 'sv-agentation'

  const enabled = import.meta.env.VITE_AGENTATION_ENABLED === 'true'
</script>

{#if enabled}
  <Agentation
    outputMode="detailed"
    includeComponentContext={true}
    includeComputedStyles={true}
    toolbarPosition="bottom-right"
  />
{/if}
