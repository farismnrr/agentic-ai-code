import { tool } from '@langchain/core/tools'
import { z } from 'zod'
import { assertSafeUrl } from './ssrf-guard'

export const curlTool = tool(
  async ({ url, method = 'GET', headers, body }) => {
    try {
      const parsedUrl = new URL(url)
      await assertSafeUrl(parsedUrl, `curl tool fetch`)
      const options: RequestInit = { method }
      if (headers) options.headers = headers
      if (body) options.body = body
      const res = await fetch(parsedUrl, options)
      const text = await res.text()
      return `Status: ${res.status}\nBody: ${text.slice(0, 10000)}`
    } catch (e: unknown) {
      return `Error: ${(e as Error).message}`
    }
  },
  {
    name: 'curl',
    description: 'Fetch a URL and return its response.',
    schema: z.object({
      url: z.string().describe('The URL to fetch.'),
      method: z.string().optional().describe('HTTP method (e.g. GET, POST).'),
      headers: z.record(z.string()).optional().describe('HTTP headers.'),
      body: z.string().optional().describe('HTTP body.')
    })
  }
)

export const searxngSearchTool = tool(
  async ({ query }) => {
    const config = useRuntimeConfig()
    try {
      const u = new URL('/search', config.searxngBaseUrl)
      u.searchParams.set('q', query)
      u.searchParams.set('format', 'json')
      const res = await fetch(u)
      if (!res.ok) return `Search failed with status: ${res.status}`
      const data = await res.json()
      const results = (data.results || []).slice(0, 10).map((r: Record<string, string>) =>
        `Title: ${r.title}\nURL: ${r.url}\nSnippet: ${r.content || r.snippet}`
      ).join('\n\n')
      return results || 'No results found.'
    } catch (e: unknown) {
      return `Error: ${(e as Error).message}`
    }
  },
  {
    name: 'searxng_search',
    description: 'Search the web using SearxNG.',
    schema: z.object({
      query: z.string().describe('The search query.')
    })
  }
)

export const langgraphTools = [curlTool, searxngSearchTool]
