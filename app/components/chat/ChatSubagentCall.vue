<script setup lang="ts">
import type { DynamicToolUIPart, ToolUIPart } from 'ai'

const props = defineProps<{ part: ToolUIPart | DynamicToolUIPart, streaming: boolean }>()
const input = computed(() => 'input' in props.part && props.part.input && typeof props.part.input === 'object' ? props.part.input as { agent?: string, task?: string } : {})
const output = computed(() => 'output' in props.part && props.part.output && typeof props.part.output === 'object' ? props.part.output as { status?: string, summary?: string, findings?: string[], evidence?: Array<{ reference?: string, detail?: string }> } : undefined)
</script>

<template>
  <UChatTool
    :text="`Subagent · ${input.agent ?? 'child'}`"
    :streaming="streaming"
    variant="card"
  >
    <div class="space-y-3 text-sm">
      <p class="text-muted">
        {{ input.task }}
      </p>
      <UBadge
        v-if="output?.status"
        size="xs"
        variant="subtle"
        :color="output.status === 'completed' ? 'success' : output.status === 'cancelled' ? 'warning' : 'error'"
      >
        {{ output.status.replaceAll('_', ' ') }}
      </UBadge>
      <p
        v-if="output?.summary"
        class="whitespace-pre-wrap"
      >
        {{ output.summary }}
      </p>
      <ul
        v-if="output?.findings?.length"
        class="list-disc pl-5"
      >
        <li
          v-for="finding in output.findings"
          :key="finding"
        >
          {{ finding }}
        </li>
      </ul>
      <div
        v-if="output?.evidence?.length"
        class="space-y-1"
      >
        <p class="font-mono text-[11px] uppercase tracking-wider text-dimmed">
          Evidence
        </p>
        <p
          v-for="item in output.evidence"
          :key="`${item.reference}-${item.detail}`"
          class="font-mono text-xs"
        >
          {{ item.reference }} — {{ item.detail }}
        </p>
      </div>
    </div>
  </UChatTool>
</template>
