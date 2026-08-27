<script setup lang="ts">
import type { Conversation } from '#shared/types/chat'
import { NATIVE_LOCAL_TERMINAL_TOOL_ID } from '#shared/utils/native-tools'

const props = defineProps<{
  modelItems: Array<{ label: string, value: string, icon: string }>
  modeItems: Array<{ label: string, value: Conversation['mode'], icon: string }>
  effortItems: Array<{ label: string, value: NonNullable<Conversation['reasoningEffort']> }>
  supportsReasoning: boolean
}>()

const mode = defineModel<Conversation['mode']>('mode', { required: true })
const modelId = defineModel<string | undefined>('modelId', { required: true })
const reasoningEffort = defineModel<NonNullable<Conversation['reasoningEffort']>>('reasoningEffort', { required: true })
const enabledToolIds = defineModel<string[]>('enabledToolIds', { default: () => [] })
const permissionMode = defineModel<Conversation['permissionMode']>('permissionMode', { default: 'manual' })

const { isConnected, checkConnection } = useRelayAgent()
const terminalEnabled = computed(() => enabledToolIds.value.includes(NATIVE_LOCAL_TERMINAL_TOOL_ID))
const agentAvailable = computed(() => terminalEnabled.value && isConnected.value)

const permissionItems = [
  { label: 'Plan mode', value: 'plan' },
  { label: 'Bypass permission', value: 'bypass' },
  { label: 'Manual permission', value: 'manual' }
] satisfies Array<{ label: string, value: Conversation['permissionMode'] }>

const availableModeItems = computed(() =>
  props.modeItems.filter(item => item.value !== 'agent' || agentAvailable.value)
)

function enforceAvailableMode() {
  if (mode.value === 'agent' && !agentAvailable.value) mode.value = 'chat'
}

watch(agentAvailable, enforceAvailableMode)
watch(terminalEnabled, (enabled) => {
  if (!enabled) {
    enforceAvailableMode()
    return
  }
  void checkConnection().then(enforceAvailableMode)
})

onMounted(() => {
  if (!terminalEnabled.value) {
    enforceAvailableMode()
    return
  }
  void checkConnection().then(enforceAvailableMode)
})
</script>

<template>
  <USelect
    v-if="terminalEnabled"
    v-model="permissionMode"
    :items="permissionItems"
    icon="i-lucide-shield-check"
    variant="ghost"
    size="sm"
  />
  <USelect
    v-model="mode"
    :items="availableModeItems"
    :icon="availableModeItems.find(item => item.value === mode)?.icon"
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
  <ChatToolPicker v-model="enabledToolIds" />
</template>
