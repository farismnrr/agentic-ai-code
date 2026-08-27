import { tool } from '@langchain/core/tools'
import { z } from 'zod'
import { createConfiguredFirstPartyRelayClient, type McpClientCallResult, type McpClientLike } from '../../mcp/client'

function contentText(result: McpClientCallResult) {
  if (result.isError) return 'Tool execution failed'

  const text = result.content.flatMap((item) => {
    if (typeof item === 'object' && item !== null && 'text' in item && typeof item.text === 'string') {
      return [item.text]
    }
    if (typeof item === 'string') return [item]
    try {
      return [JSON.stringify(item)]
    } catch {
      return []
    }
  }).filter((item): item is string => Boolean(item)).join('\n')

  return text || 'Tool execution failed'
}

async function invokeRelay(client: McpClientLike, name: string, args: Record<string, unknown>) {
  try {
    return contentText(await client.callTool({ name, arguments: args }))
  } catch {
    // Keep remote relay and provider details out of user-visible LangGraph
    // tool output. The relay/client boundary records the private diagnostics.
    return 'Tool execution failed'
  }
}

/**
 * Builds the chat-mode network tools over the separately deployed first-party
 * relay. The Nuxt process has no Rust binary or native-tool workspace in its
 * image; an unset relay configuration degrades to no executable tools.
 */
export async function buildLanggraphTools() {
  const relay = await createConfiguredFirstPartyRelayClient()
  if (!relay) {
    return {
      tools: [],
      search: async () => 'Tool execution failed',
      close: async () => {}
    }
  }

  return {
    tools: [
      tool(
        async ({ url, method = 'GET', headers, body }) => invokeRelay(relay, 'http_fetch', {
          url,
          method,
          ...(headers ? { headers } : {}),
          ...(body ? { data: body } : {})
        }),
        {
          name: 'curl',
          description: 'Fetch a URL through the separately deployed coding relay and return its response.',
          schema: z.object({
            url: z.string().describe('The URL to fetch.'),
            method: z.string().optional().describe('HTTP method (e.g. GET, POST).'),
            headers: z.record(z.string(), z.string()).optional().describe('HTTP headers.'),
            body: z.string().optional().describe('HTTP body.')
          })
        }
      ),
      tool(
        async ({ query }) => invokeRelay(relay, 'web_search', { query }),
        {
          name: 'searxng_search',
          description: 'Search the web through the separately deployed coding relay.',
          schema: z.object({
            query: z.string().describe('The search query.')
          })
        }
      )
    ],
    search: async (query: string) => invokeRelay(relay, 'web_search', { query }),
    close: () => relay.close()
  }
}
