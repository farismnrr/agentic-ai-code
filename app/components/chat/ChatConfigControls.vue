<script setup lang="ts">
import type { Conversation } from '#shared/types/chat'

defineProps<{
  modelItems: Array<{ label: string, value: string, icon: string }>
  modeItems: Array<{ label: string, value: Conversation['mode'], icon: string }>
  effortItems: Array<{ label: string, value: NonNullable<Conversation['reasoningEffort']> }>
  supportsReasoning: boolean
  showTools?: boolean
  approvals?: Record<string, 'always' | 'never'>
}>()

const mode = defineModel<Conversation['mode']>('mode', { required: true })
const modelId = defineModel<string | undefined>('modelId', { required: true })
const reasoningEffort = defineModel<NonNullable<Conversation['reasoningEffort']>>('reasoningEffort', { required: true })
const enabledToolIds = defineModel<string[]>('enabledToolIds', { default: () => [] })
const permissionMode = defineModel<Conversation['permissionMode']>('permissionMode', { default: 'manual' })
const emit = defineEmits<{ updateApprovals: [approvals: Record<string, 'always' | 'never'>] }>()

const permissionItems = [
  { label: 'Plan / read-only', value: 'plan' },
  { label: 'Workspace', value: 'workspace' },
  { label: 'Autonomous sandboxed', value: 'autonomous' },
  { label: 'Manual approval', value: 'manual' }
] satisfies Array<{ label: string, value: Conversation['permissionMode'] }>
</script>

<template>
  <USelect
    v-model="permissionMode"
    :items="permissionItems"
    icon="i-lucide-shield-check"
    variant="ghost"
    size="sm"
  />
  <USelect
    v-model="mode"
    :items="modeItems"
    :icon="modeItems.find(item => item.value === mode)?.icon"
    variant="ghost"
    size="sm"
  />
  <USelect
    v-model="modelId"
    :items="modelItems"
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
  <ChatToolPicker
    v-if="showTools"
    v-model="enabledToolIds"
    :approvals="approvals"
    @update:approvals="emit('updateApprovals', $event)"
  />
</template>
