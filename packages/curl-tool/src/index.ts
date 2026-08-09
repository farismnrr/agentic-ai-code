import { tool } from '@langchain/core/tools'
import { z } from 'zod'

export const createCurlTool = ({ assertSafeUrl }: { assertSafeUrl: (url: URL, context: string) => Promise<void> }) => {
  return tool(
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
        headers: z.record(z.string(), z.string()).optional().describe('HTTP headers.'),
        body: z.string().optional().describe('HTTP body.')
      })
    }
  )
}
