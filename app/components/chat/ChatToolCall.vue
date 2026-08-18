<script setup lang="ts">
import type { DynamicToolUIPart, ToolUIPart } from 'ai'
import { resolveMcpToolFromModelName } from '#shared/utils/mcp-tool-identity'
import { categoryLabel, presentToolOutput, safeInputSummary, toolCategory } from '../../utils/tool-presentation'

const props = defineProps<{ part: ToolUIPart | DynamicToolUIPart, toolName: string, streaming: boolean }>()
const { toolsById } = useMcpServers()
const tool = computed(() => resolveMcpToolFromModelName(props.toolName, Object.values(toolsById.value)))
const server = computed(() => tool.value?.serverId)
const displayName = computed(() => tool.value?.name ?? props.toolName)
const category = computed(() => toolCategory(displayName.value))
const input = computed(() => safeInputSummary(props.part.input))
const output = computed(() => presentToolOutput('output' in props.part ? props.part.output : undefined))
const errorText = computed(() => {
  const value = 'errorText' in props.part ? props.part.errorText : undefined
  if (!value) return undefined
  return value.length > 1000 ? `${value.slice(0, 1000)}…` : value
})
</script>

<template>
  <UChatTool
    :text="server ? `${server} · ${displayName}` : displayName"
    :streaming="streaming"
    variant="card"
  >
    <div class="space-y-3 text-sm">
      <div class="flex flex-wrap gap-2">
        <UBadge
          :label="categoryLabel(category)"
          size="xs"
          color="neutral"
          variant="subtle"
        />
        <UBadge
          v-if="part.state === 'output-denied'"
          label="Denied"
          size="xs"
          color="warning"
          variant="subtle"
        />
      </div>

      <dl
        v-if="input.rows.length"
        class="grid gap-1.5 text-xs sm:grid-cols-2"
      >
        <div
          v-for="row in input.rows"
          :key="row.label"
          class="min-w-0"
        >
          <dt class="text-dimmed">
            {{ row.label }}
          </dt>
          <dd
            class="truncate font-mono"
            :title="row.value"
          >
            {{ row.value }}
          </dd>
        </div>
      </dl>
      <p
        v-if="input.hiddenFields"
        class="text-xs text-dimmed"
      >
        {{ input.hiddenFields }} sensitive/noisy input field{{ input.hiddenFields === 1 ? '' : 's' }} hidden.
      </p>

      <div
        v-if="errorText"
        class="rounded-md bg-elevated p-2"
      >
        <p class="font-mono text-[11px] uppercase tracking-wider text-error">
          Failed
        </p>
        <p class="mt-1 text-xs text-error">
          {{ errorText }}
        </p>
      </div>
      <div
        v-else-if="output"
        class="space-y-2"
      >
        <p class="text-xs text-muted">
          {{ output.summary }}
        </p>
        <details
          v-if="output.preview"
          class="rounded-md bg-elevated p-2"
        >
          <summary class="cursor-pointer text-xs font-medium">
            {{ output.previewLabel }}
          </summary>
          <pre class="mt-2 max-h-80 overflow-auto whitespace-pre-wrap font-mono text-xs">{{ output.preview }}</pre>
        </details>
        <p
          v-if="output.truncated"
          class="text-xs text-warning"
        >
          Preview is bounded{{ output.continuation ? '; continuation is available from the tool result.' : '.' }}
        </p>
      </div>
      <p
        v-else-if="part.state === 'output-denied'"
        class="text-muted"
      >
        Denied — the tool was not run.
      </p>
    </div>
  </UChatTool>
</template>
