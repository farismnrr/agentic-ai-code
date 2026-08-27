<script setup lang="ts">
import type { Conversation } from '#shared/types/chat'

const props = defineProps<{
  modelItems: Array<{ label: string, value: string, icon: string }>
  modeItems: Array<{ label: string, value: Conversation['mode'], icon: string }>
  effortItems: Array<{ label: string, value: NonNullable<Conversation['reasoningEffort']> }>
  supportsReasoning: boolean
  agentAvailable: boolean
}>()

const mode = defineModel<Conversation['mode']>('mode', { required: true })
const modelId = defineModel<string | undefined>('modelId', { required: true })
const reasoningEffort = defineModel<NonNullable<Conversation['reasoningEffort']>>('reasoningEffort', { required: true })
const permissionMode = defineModel<Conversation['permissionMode']>('permissionMode', { default: 'manual' })

const permissionItems = [
  { label: 'Plan mode', value: 'plan' },
  { label: 'Bypass permission', value: 'bypass' },
  { label: 'Manual permission', value: 'manual' }
] satisfies Array<{ label: string, value: Conversation['permissionMode'] }>

const availableModeItems = computed(() =>
  props.modeItems.filter(item => item.value !== 'agent' || props.agentAvailable)
)

function enforceAvailableMode() {
  if (mode.value === 'agent' && !props.agentAvailable) mode.value = 'chat'
}

watch(() => props.agentAvailable, enforceAvailableMode, { immediate: true })
</script>

<template>
  <USelect
    v-model="mode"
    :items="availableModeItems"
    :icon="availableModeItems.find(item => item.value === mode)?.icon"
    variant="ghost"
    size="sm"
  />
  <USelect
    v-if="mode === 'agent' && agentAvailable"
    v-model="permissionMode"
    :items="permissionItems"
    icon="i-lucide-shield-check"
    variant="ghost"
    size="sm"
  />
  <USelect
    v-model="modelId"
    :items="modelItems"
    placeholder="Choose a model"
    icon="i-lucide-box"
    variant="ghost"
    size="sm"
  />
  <USelect
    v-if="supportsReasoning"
    v-model="reasoningEffort"
    :items="effortItems"
    variant="ghost"
    size="sm"
  />
</template>
