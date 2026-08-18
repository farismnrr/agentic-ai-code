<script setup lang="ts">
import type { DynamicToolUIPart, ToolUIPart } from 'ai'
import { safeInputSummary } from '../../utils/tool-presentation'

const props = defineProps<{ part: ToolUIPart | DynamicToolUIPart, streaming: boolean, toolName?: string }>()
const rawInput = computed(() => 'input' in props.part ? props.part.input : undefined)
const input = computed(() => rawInput.value && typeof rawInput.value === 'object' ? rawInput.value as { agent?: string, task?: string, isolation?: string, task_id?: string } : {})
const output = computed(() => 'output' in props.part && props.part.output && typeof props.part.output === 'object'
  ? props.part.output as {
    status?: string
    state?: string
    summary?: string
    progress_summary?: string
    findings?: string[]
    validation?: string[]
    remaining_risks?: string[]
    evidence?: Array<{ reference?: string, detail?: string }>
    branch?: string
    worktree_path?: string
    task_id?: string
    cleanup?: string
    usage?: { turns?: number, tool_calls?: number, output_tokens?: number }
  }
  : undefined)
const summary = computed(() => safeInputSummary(rawInput.value))
const state = computed(() => output.value?.state ?? output.value?.status)
const isBackground = computed(() => props.toolName?.startsWith('background_') === true || props.toolName?.startsWith('agent_task_') === true)
const label = computed(() => isBackground.value ? `Background · ${input.value.agent ?? 'agent'}` : `Subagent · ${input.value.agent ?? 'child'}`)
const stateColor = computed(() => state.value === 'completed' ? 'success' : state.value === 'cancelled' || state.value === 'cancelling' ? 'warning' : ['failed', 'blocked', 'rejected'].includes(state.value ?? '') ? 'error' : 'neutral')
</script>

<template>
  <UChatTool
    :text="label"
    :streaming="streaming"
    variant="card"
  >
    <div class="space-y-3 text-sm">
      <div class="flex flex-wrap items-center gap-2">
        <UBadge
          v-if="state"
          size="xs"
          variant="subtle"
          :color="stateColor"
        >
          {{ state.replaceAll('_', ' ') }}
        </UBadge>
        <UBadge
          v-if="input.isolation"
          size="xs"
          variant="subtle"
          color="neutral"
        >
          {{ input.isolation.replaceAll('_', ' ') }}
        </UBadge>
        <span
          v-if="output?.usage?.tool_calls != null"
          class="text-xs text-dimmed"
        >{{ output.usage.tool_calls }} tool calls</span>
      </div>

      <p
        v-if="output?.progress_summary"
        class="text-muted"
      >
        {{ output.progress_summary }}
      </p>

      <dl
        v-if="summary.rows.length"
        class="grid gap-1.5 text-xs sm:grid-cols-2"
      >
        <div
          v-for="row in summary.rows.filter(row => row.label !== 'task')"
          :key="row.label"
        >
          <dt class="text-dimmed">
            {{ row.label }}
          </dt><dd
            class="truncate font-mono"
            :title="row.value"
          >
            {{ row.value }}
          </dd>
        </div>
      </dl>
      <p
        v-if="summary.hiddenFields"
        class="text-xs text-dimmed"
      >
        {{ summary.hiddenFields }} sensitive/noisy field{{ summary.hiddenFields === 1 ? '' : 's' }} hidden.
      </p>

      <div
        v-if="output?.branch || output?.worktree_path"
        class="rounded-md bg-elevated p-2 text-xs"
      >
        <p class="font-medium">
          Isolated worktree
        </p>
        <p
          v-if="output.branch"
          class="mt-1 font-mono"
        >
          {{ output.branch }}
        </p>
        <p
          v-if="output.worktree_path"
          class="mt-1 text-dimmed"
        >
          Path identity is available but kept compact.
        </p>
        <p
          v-if="output.cleanup"
          class="mt-1 text-dimmed"
        >
          Cleanup: {{ output.cleanup.replaceAll('_', ' ') }}
        </p>
      </div>

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
          v-for="finding in output.findings.slice(0, 12)"
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
          v-for="item in output.evidence.slice(0, 12)"
          :key="`${item.reference}-${item.detail}`"
          class="font-mono text-xs"
        >
          {{ item.reference }} — {{ item.detail }}
        </p>
      </div>
      <div
        v-if="output?.validation?.length"
        class="space-y-1"
      >
        <p class="text-xs font-medium">
          Validation
        </p><p
          v-for="item in output.validation.slice(0, 8)"
          :key="item"
          class="text-xs text-muted"
        >
          {{ item }}
        </p>
      </div>
      <div
        v-if="output?.remaining_risks?.length"
        class="space-y-1"
      >
        <p class="text-xs font-medium text-warning">
          Remaining / unproven
        </p><p
          v-for="item in output.remaining_risks.slice(0, 8)"
          :key="item"
          class="text-xs text-muted"
        >
          {{ item }}
        </p>
      </div>
    </div>
  </UChatTool>
</template>
