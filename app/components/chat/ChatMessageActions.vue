<script setup lang="ts">
import { getTextFromMessage } from '@nuxt/ui/utils/ai'
import type { UIMessage } from '#shared/types/chat'

const props = defineProps<{ message: UIMessage, feedback?: 'up' | 'down' }>()
const emit = defineEmits<{
  edit: [message: { id: string, text: string }]
  regenerate: []
  rate: [value: 'up' | 'down']
}>()
const toast = useToast()

async function copy() {
  await navigator.clipboard.writeText(getTextFromMessage(props.message))
  toast.add({ title: 'Copied', icon: 'i-lucide-check', color: 'success' })
}
</script>

<template>
  <UButton
    icon="i-lucide-copy"
    color="neutral"
    variant="ghost"
    size="xs"
    aria-label="Copy message"
    @click="copy"
  />
  <UButton
    v-if="message.role === 'user'"
    icon="i-lucide-pencil"
    color="neutral"
    variant="ghost"
    size="xs"
    aria-label="Edit and resend"
    @click="emit('edit', { id: message.id, text: getTextFromMessage(message) })"
  />
  <template v-if="message.role === 'assistant'">
    <UButton
      icon="i-lucide-refresh-cw"
      color="neutral"
      variant="ghost"
      size="xs"
      aria-label="Regenerate"
      @click="emit('regenerate')"
    />
    <UButton
      icon="i-lucide-thumbs-up"
      :color="feedback === 'up' ? 'primary' : 'neutral'"
      variant="ghost"
      size="xs"
      aria-label="Good response"
      @click="emit('rate', 'up')"
    />
    <UButton
      icon="i-lucide-thumbs-down"
      :color="feedback === 'down' ? 'error' : 'neutral'"
      variant="ghost"
      size="xs"
      aria-label="Bad response"
      @click="emit('rate', 'down')"
    />
  </template>
</template>
