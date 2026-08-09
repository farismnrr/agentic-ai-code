import { tool } from '@langchain/core/tools'
import { z } from 'zod'

export const createSearxngSearchTool = ({ baseUrl }: { baseUrl: string }) => {
  return tool(
    async ({ query }) => {
      try {
        const u = new URL('/search', baseUrl)
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
}
