<script setup lang="ts">
/* eslint-disable @stylistic/max-statements-per-line */
import type { AgentTask } from '#shared/types/chat'

const props = defineProps<{ conversationId: string, visible: boolean }>()
const tasks = ref<AgentTask[]>([])
const load = async () => { if (!props.visible) return; try { const result = await $fetch<{ tasks: AgentTask[] }>(`/api/conversations/${props.conversationId}/tasks`); tasks.value = result.tasks } catch { tasks.value = [] } }
let timer: ReturnType<typeof setInterval> | undefined
watch(() => [props.conversationId, props.visible], () => { if (timer) clearInterval(timer); void load(); if (props.visible) timer = setInterval(load, 1000) }, { immediate: true })
onBeforeUnmount(() => { if (timer) clearInterval(timer) })
const color = (status: AgentTask['status']) => status === 'completed' ? 'success' : status === 'blocked' ? 'error' : status === 'in_progress' ? 'primary' : 'neutral'
</script>

<template>
  <div
    v-if="visible && tasks.length"
    class="mb-3 rounded-lg border border-default bg-elevated/30 p-3 max-w-3xl mx-auto"
    aria-label="Ephemeral task progress"
  >
    <div class="flex items-center justify-between mb-2">
      <span class="text-xs font-medium">Progress</span><span class="text-[10px] text-dimmed">Ephemeral · not verification</span>
    </div>
    <div class="space-y-1.5">
      <div
        v-for="task in tasks"
        :key="task.id"
        class="flex items-start gap-2 text-xs"
      >
        <UBadge
          size="xs"
          variant="subtle"
          :color="color(task.status)"
        >
          {{ task.status.replace('_', ' ') }}
        </UBadge>
        <span class="min-w-0">{{ task.title }}<span
          v-if="task.short_note"
          class="text-dimmed"
        > — {{ task.short_note }}</span></span>
      </div>
    </div>
  </div>
</template>
