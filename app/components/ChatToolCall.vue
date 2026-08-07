<script setup lang="ts">
import type { DynamicToolUIPart, ToolUIPart } from 'ai'
import { mcpToolsById } from '~/utils/fixtures/mcp-servers'

// `isToolUIPart` narrows to either shape — MCP tools arrive as dynamic ones,
// so accepting only `ToolUIPart` would reject exactly the case we care about.
const props = defineProps<{
  part: ToolUIPart | DynamicToolUIPart
  toolName: string
  streaming: boolean
}>()

/** Tool ids are `<serverId>.<name>`, so the server is recoverable from the name. */
const server = computed(() =>
  Object.values(mcpToolsById).find(t => t.name === props.toolName)?.serverId
)

const input = computed(() => {
  const value = props.part.input
  return value ? JSON.stringify(value, null, 2) : undefined
})

const output = computed(() => {
  const value = 'output' in props.part ? props.part.output : undefined
  return value ? JSON.stringify(value, null, 2) : undefined
})

const errorText = computed(() =>
  'errorText' in props.part ? props.part.errorText : undefined
)
</script>

<template>
  <UChatTool
    :text="server ? `${server} · ${toolName}` : toolName"
    :streaming="streaming"
    variant="card"
  >
    <div class="space-y-3 text-sm">
      <div v-if="input">
        <p class="mb-1 text-xs font-medium text-dimmed">
          Arguments
        </p>
        <pre class="overflow-x-auto rounded-md bg-elevated p-2 text-xs">{{ input }}</pre>
      </div>

      <div v-if="errorText">
        <p class="mb-1 text-xs font-medium text-error">
          Error
        </p>
        <p class="text-error">
          {{ errorText }}
        </p>
      </div>

      <div v-else-if="output">
        <p class="mb-1 text-xs font-medium text-dimmed">
          Result
        </p>
        <pre class="overflow-x-auto rounded-md bg-elevated p-2 text-xs">{{ output }}</pre>
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
