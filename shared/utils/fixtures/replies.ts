import type { UIMessageChunk } from 'ai'
import type { McpTool } from '#shared/types/chat'

/** Stand-in for a real tool, used only by the landing page's self-playing demo. */
const mcpToolsById: Record<string, McpTool> = {
  github: {
    id: 'github.search_repositories',
    serverId: 'github',
    name: 'search_repositories',
    description: 'Search GitHub repositories.',
    sampleInput: { query: 'nuxt UI' }
  }
}

/**
 * Canned assistant responses, expressed as the AI SDK's own `UIMessageChunk`
 * stream. Building real chunks rather than finished strings is what lets the
 * UI exercise streaming, reasoning, tool calls and errors for real — and what
 * makes swapping in a live endpoint a no-op for every component.
 */

/** Split text into deltas roughly the size of a token, so streaming looks real. */
function textDeltas(id: string, text: string): UIMessageChunk[] {
  const pieces = text.match(/\S+\s*/g) ?? [text]
  return [
    { type: 'text-start', id },
    ...pieces.map((delta): UIMessageChunk => ({ type: 'text-delta', id, delta })),
    { type: 'text-end', id }
  ]
}

function reasoningDeltas(id: string, text: string): UIMessageChunk[] {
  const pieces = text.match(/\S+\s*/g) ?? [text]
  return [
    { type: 'reasoning-start', id },
    ...pieces.map((delta): UIMessageChunk => ({ type: 'reasoning-delta', id, delta })),
    { type: 'reasoning-end', id }
  ]
}

/**
 * A tool call that pauses for approval. The SDK models this natively:
 * `tool-approval-request` suspends the part in an `approval-requested` state
 * until the UI answers via `addToolApprovalResponse()`.
 */
function toolCall(tool: McpTool, output: unknown, opts: { requireApproval: boolean }): UIMessageChunk[] {
  const toolCallId = `call_${Math.random().toString(36).slice(2, 10)}`

  const chunks: UIMessageChunk[] = [
    { type: 'tool-input-start', toolCallId, toolName: tool.name, title: tool.name },
    {
      type: 'tool-input-delta',
      toolCallId,
      inputTextDelta: JSON.stringify(tool.sampleInput)
    },
    {
      type: 'tool-input-available',
      toolCallId,
      toolName: tool.name,
      input: tool.sampleInput,
      title: tool.name
    }
  ]

  if (opts.requireApproval) {
    chunks.push({
      type: 'tool-approval-request',
      approvalId: `approval_${toolCallId}`,
      toolCallId
    })
  }

  chunks.push({ type: 'tool-output-available', toolCallId, output })

  return chunks
}

export interface Scenario {
  id: string
  /** Matched against the user's prompt, lowercased. */
  matches: (prompt: string) => boolean
  build: (ctx: { enabledToolIds: string[] }) => UIMessageChunk[]
}

const CODE_REPLY = `Here's a minimal Nuxt UI chat prompt:

\`\`\`vue
<script setup lang="ts">
const input = ref('')
</script>

<template>
  <UChatPrompt v-model="input" @submit="onSubmit">
    <UChatPromptSubmit :status="status" />
  </UChatPrompt>
</template>
\`\`\`

\`UChatPromptSubmit\` reads \`status\` and swaps between send, stop and retry on its own — you don't wire those separately.`

export const scenarios: Scenario[] = [
  {
    id: 'tool',
    matches: prompt => /file|repo|github|search|list|query|tool|mcp/.test(prompt),
    build: ({ enabledToolIds }) => {
      const tool = mcpToolsById[enabledToolIds[0] ?? '']
      if (!tool) {
        return textDeltas('t1', 'No MCP tools are enabled for this conversation, so I answered without one.')
      }
      return [
        ...textDeltas('t1', `I'll use \`${tool.name}\` from the ${tool.serverId} server for this.`),
        ...toolCall(tool, { ok: true, summary: `${tool.name} returned 3 results.` }, { requireApproval: true }),
        ...textDeltas('t2', 'Done — the tool returned three results, summarised above.')
      ]
    }
  },
  {
    id: 'reasoning',
    matches: prompt => /why|explain|how come|reason|think/.test(prompt),
    build: () => [
      ...reasoningDeltas(
        'r1',
        'The user is asking for an explanation rather than an action. I should lay out the cause before the conclusion, and keep it short.'
      ),
      ...textDeltas(
        't1',
        'Short version: the components read their state from the AI SDK, so the transport is the only piece that needs replacing when a backend arrives.'
      )
    ]
  },
  {
    id: 'code',
    matches: prompt => /code|example|snippet|component|vue|show me/.test(prompt),
    build: () => textDeltas('t1', CODE_REPLY)
  },
  {
    id: 'error',
    matches: prompt => /fail|error|break|throw/.test(prompt),
    build: () => [
      { type: 'error', errorText: 'Mock failure: the model provider returned 503.' }
    ]
  },
  {
    id: 'default',
    matches: () => true,
    build: () => textDeltas(
      't1',
      'This is a mock reply — there is no model behind it yet. Try asking about a **file** or **repo** to see an MCP tool call, or **why** something works to see a reasoning block.'
    )
  }
]

export function pickScenario(prompt: string): Scenario {
  const normalised = prompt.toLowerCase()
  return scenarios.find(s => s.matches(normalised)) ?? scenarios[scenarios.length - 1]!
}
