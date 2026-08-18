<script setup lang="ts">
import type { Conversation } from '#shared/types/chat'
import type { ContextInspectorData } from '../../utils/context-usage'
import { presentContextUsage } from '../../utils/context-usage'

const props = defineProps<{
  conversationId: string | undefined
  conversation: Conversation | undefined
}>()

const context = ref<ContextInspectorData | null>(null)
const requestFailed = ref(false)
let requestSerial = 0

const presentation = computed(() => presentContextUsage(context.value))

async function loadContext() {
  const id = props.conversationId
  const serial = ++requestSerial
  if (!id) {
    context.value = null
    requestFailed.value = false
    return
  }

  try {
    const fetchContext = import.meta.server ? useRequestFetch() : $fetch
    const data = await fetchContext<ContextInspectorData>(`/api/conversations/${encodeURIComponent(id)}/context`)
    if (serial === requestSerial) {
      context.value = data
      requestFailed.value = false
    }
  } catch {
    if (serial === requestSerial) {
      context.value = null
      requestFailed.value = true
    }
  }
}

// Conversation metadata is refreshed once after each completed turn by the
// chat composable. Watching its identity/timestamp/length refreshes the
// inspector at those lifecycle points without polling every token or second.
watch(
  () => [props.conversationId, props.conversation?.updatedAt, props.conversation?.messages.length],
  () => { void loadContext() },
  { immediate: true }
)
</script>

<template>
  <div
    class="flex items-center gap-1.5 px-1"
    :title="presentation.detail ?? undefined"
  >
    <template v-if="presentation.percent != null">
      <UProgress
        :model-value="presentation.percent"
        size="xs"
        :color="presentation.percent > 85 ? 'error' : 'neutral'"
        class="w-16"
      />
      <span class="text-[11px] text-muted whitespace-nowrap font-mono">{{ presentation.label }}</span>
    </template>
    <span
      v-else-if="requestFailed || props.conversationId"
      class="text-[11px] text-muted whitespace-nowrap"
    >{{ presentation.label }}</span>
  </div>
</template>
