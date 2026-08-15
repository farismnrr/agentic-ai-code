<script setup lang="ts">
const editing = defineModel<{ id: string, text: string } | null>()
const emit = defineEmits<{ send: [message: { id: string, text: string }] }>()

function confirm() {
  if (!editing.value?.text.trim()) return
  const message = editing.value
  editing.value = null
  emit('send', { ...message, text: message.text.trim() })
}
</script>

<template>
  <UModal
    :open="editing !== null"
    title="Edit message"
    description="Everything after this message will be replaced."
    @update:open="editing = null"
  >
    <template #body>
      <UTextarea
        v-if="editing"
        v-model="editing.text"
        :rows="4"
        autoresize
        autofocus
        class="w-full"
      />
    </template>
    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton
          label="Cancel"
          color="neutral"
          variant="ghost"
          @click="editing = null"
        /><UButton
          label="Send"
          @click="confirm"
        />
      </div>
    </template>
  </UModal>
</template>
