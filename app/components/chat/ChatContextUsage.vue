<script setup lang="ts">
import type { Conversation } from '#shared/types/chat'

const props = defineProps<{
  conversation: Conversation | undefined
  modelId: string | undefined
}>()

const { models } = useModels()
const currentModel = computed(() => models.value.find(m => m.id === props.modelId))

const budget = computed(() => {
  if (!currentModel.value?.contextWindow) return 0
  return currentModel.value.contextWindow - (currentModel.value.maxOutputTokens ?? 0)
})

const used = computed(() => props.conversation?.lastMeasuredTokens ?? 0)
const exact = computed(() => props.conversation?.lastMeasuredTokens != null)

const percent = computed(() => {
  if (budget.value <= 0) return 0
  return Math.min(100, Math.round((used.value / budget.value) * 100))
})
</script>

<template>
  <div
    v-if="currentModel?.contextWindow"
    class="flex items-center gap-1.5 px-1"
  >
    <UProgress
      :model-value="percent"
      size="xs"
      :color="percent > 85 ? 'error' : 'neutral'"
      class="w-16"
    />
    <span class="text-[11px] text-muted whitespace-nowrap font-mono">{{ percent }}% {{ exact ? 'accounted' : 'estimated' }} context</span>
  </div>
</template>
