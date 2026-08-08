<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'

const props = defineProps<{
  open: boolean
  filter: string
}>()

const emit = defineEmits<{
  select: [trigger: string]
  close: []
}>()

const allItems = [
  { trigger: 'search', label: '@search', description: 'Force this turn to search the web', icon: 'i-lucide-search' }
]

const items = computed(() => {
  const f = props.filter.toLowerCase()
  return allItems.filter(item => item.trigger.toLowerCase().startsWith(f))
})

const highlighted = ref(0)

watch(() => props.open, (isOpen) => {
  if (isOpen) {
    highlighted.value = 0
  }
})

watch(() => props.filter, () => {
  highlighted.value = 0
})

function select(index: number) {
  if (items.value[index]) {
    emit('select', items.value[index].trigger)
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (!props.open || items.value.length === 0) return
  if (document.activeElement?.tagName !== 'TEXTAREA') return

  if (e.key === 'ArrowDown') {
    e.preventDefault()
    e.stopPropagation()
    highlighted.value = (highlighted.value + 1) % items.value.length
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    e.stopPropagation()
    highlighted.value = (highlighted.value - 1 + items.value.length) % items.value.length
  } else if (e.key === 'Enter' || e.key === 'Tab') {
    e.preventDefault()
    e.stopPropagation()
    select(highlighted.value)
  } else if (e.key === 'Escape') {
    e.preventDefault()
    e.stopPropagation()
    emit('close')
  }
}

onMounted(() => {
  // Use capture phase to intercept keys before UChatPrompt or textarea gets them
  window.addEventListener('keydown', handleKeydown, true)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown, true)
})
</script>

<template>
  <div
    v-if="open && items.length > 0"
    class="absolute bottom-full left-0 mb-2 w-72 rounded-md bg-elevated shadow-lg ring-1 ring-[var(--ui-border)] p-1 z-50 overflow-hidden"
  >
    <button
      v-for="(item, index) in items"
      :key="item.trigger"
      class="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm transition-colors"
      :class="index === highlighted ? 'bg-[var(--ui-bg-elevated)]' : 'hover:bg-[var(--ui-bg-elevated)]'"
      @click="select(index)"
      @mousemove="highlighted = index"
    >
      <UIcon
        :name="item.icon"
        class="text-muted shrink-0"
      />
      <div class="flex flex-col overflow-hidden">
        <span class="font-medium text-default truncate">{{ item.label }}</span>
        <span class="text-xs text-muted truncate">{{ item.description }}</span>
      </div>
    </button>
  </div>
</template>
