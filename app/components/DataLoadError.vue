<script setup lang="ts">
interface Props {
  title?: string
  description?: string
  icon?: string
}

withDefaults(defineProps<Props>(), {
  title: 'Failed to load data',
  description: 'An error occurred while fetching data from the server.',
  icon: 'i-lucide-alert-circle'
})

defineEmits<{
  retry: []
}>()
</script>

<template>
  <div class="flex flex-col items-center justify-center p-6 text-center rounded-lg border border-dashed border-error/30 bg-error/5 my-2">
    <UIcon
      :name="icon"
      class="size-8 text-error mb-2"
    />
    <h3 class="text-sm font-semibold text-highlighted mb-1">
      {{ title }}
    </h3>
    <p class="text-xs text-muted max-w-xs mb-3">
      {{ description }}
    </p>
    <UButton
      label="Retry"
      icon="i-lucide-refresh-cw"
      color="neutral"
      variant="outline"
      size="xs"
      @click="$emit('retry')"
    />
  </div>
</template>
