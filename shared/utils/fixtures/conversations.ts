import type { Conversation, UIMessage } from '#shared/types/chat'
import { defaultEnabledToolIds } from './mcp-servers.js'
import { defaultModelId } from './models'

function message(id: string, role: UIMessage['role'], text: string): UIMessage {
  return { id, role, parts: [{ type: 'text', text }] }
}

const HOUR = 60 * 60 * 1000
const DAY = 24 * HOUR
const now = Date.now()

/**
 * Seed conversations so the sidebar has something to group and the chat view
 * has history to scroll. Timestamps are relative to load so the Today /
 * Previous 7 days grouping stays correct whenever this runs.
 */
export const seedConversations: Conversation[] = [
  {
    id: 'seed-nuxt-ui',
    title: 'Streaming chat components',
    createdAt: now - 2 * HOUR,
    updatedAt: now - 2 * HOUR,
    workspaceId: 'seed-ws',
    modelId: defaultModelId,
    enabledToolIds: defaultEnabledToolIds,
    approvals: {},
    messages: [
      message('m1', 'user', 'Which Nuxt UI components handle streaming chat?'),
      message(
        'm2',
        'assistant',
        'The chat family: `UChatMessages` for the list, `UChatMessage` for a bubble, `UChatPrompt` and `UChatPromptSubmit` for input, plus `UChatReasoning` and `UChatTool` for reasoning blocks and tool calls.'
      )
    ]
  },
  {
    id: 'seed-mcp',
    title: 'Listing MCP tools',
    createdAt: now - 6 * HOUR,
    updatedAt: now - 5 * HOUR,
    workspaceId: 'seed-ws',
    modelId: defaultModelId,
    enabledToolIds: defaultEnabledToolIds,
    approvals: {},
    messages: [
      message('m1', 'user', 'List the tools the github server exposes.'),
      message(
        'm2',
        'assistant',
        'Three: `search_repositories`, `get_issue`, and `list_pull_requests`.'
      )
    ]
  },
  {
    id: 'seed-tailwind',
    title: 'Theme tokens in Tailwind 4',
    createdAt: now - 3 * DAY,
    updatedAt: now - 3 * DAY,
    workspaceId: 'seed-ws',
    modelId: 'claude-haiku-4-5',
    enabledToolIds: [],
    approvals: {},
    messages: [
      message('m1', 'user', 'Why is there no tailwind.config.js?'),
      message(
        'm2',
        'assistant',
        'Tailwind 4 moved configuration into CSS. Theme extensions go in `app/assets/css/main.css` under `@theme`.'
      )
    ]
  }
]
