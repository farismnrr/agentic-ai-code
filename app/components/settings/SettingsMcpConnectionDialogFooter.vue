<script setup lang="ts">
defineProps<{
  isEditing: boolean
  scanning: boolean
  saving: boolean
  canCreate: boolean
  canSave: boolean
  scanFresh: boolean
}>()

const emit = defineEmits<{
  cancel: []
  scan: []
  save: []
}>()
</script>

<template>
  <div class="flex w-full items-center justify-between gap-3">
    <UButton
      label="Cancel"
      color="neutral"
      variant="ghost"
      @click="emit('cancel')"
    />

    <UButton
      v-if="!isEditing"
      label="Create"
      icon="i-lucide-plus"
      type="submit"
      form="mcp-connection-form"
      :loading="scanning || saving"
      :disabled="!canCreate"
    />

    <div
      v-else
      class="flex gap-2"
    >
      <UButton
        :label="scanFresh ? 'Scan again' : 'Scan tools'"
        icon="i-lucide-scan-search"
        color="neutral"
        variant="outline"
        type="submit"
        form="mcp-connection-form"
        :loading="scanning"
        :disabled="saving"
        @click="emit('scan')"
      />
      <UButton
        label="Save changes"
        icon="i-lucide-save"
        type="submit"
        form="mcp-connection-form"
        :loading="saving"
        :disabled="!canSave || scanning"
        @click="emit('save')"
      />
    </div>
  </div>
</template>
