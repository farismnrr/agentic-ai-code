<script setup lang="ts">
const acknowledged = defineModel<boolean>({ default: false })
</script>

<template>
  <div class="overflow-hidden rounded-lg border border-warning/40">
    <div class="flex items-start gap-3 bg-warning/15 px-4 py-3">
      <UIcon
        name="i-lucide-triangle-alert"
        class="mt-0.5 size-5 shrink-0 text-warning"
      />
      <div>
        <p class="text-sm font-medium text-highlighted">
          Custom MCP servers introduce risk
        </p>
        <p class="mt-0.5 text-xs leading-5 text-muted">
          Only connect servers you trust. Their tools may read data or perform actions when you approve them.
        </p>
      </div>
    </div>
    <div class="bg-default px-4 py-4">
      <UCheckbox
        v-model="acknowledged"
        label="I understand and want to continue"
        description="This MCP server is not reviewed by AI Code. Review its tools and permissions before enabling them in a conversation."
        :ui="{ label: 'font-medium' }"
      />
    </div>
  </div>
</template>
