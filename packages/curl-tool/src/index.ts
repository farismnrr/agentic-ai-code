// @ts-expect-error - Ignore module resolution errors in CI
import { tool } from '@langchain/core/tools'
import { z } from 'zod'
import { execa } from 'execa'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export const createCurlTool = ({ assertSafeUrl }: { assertSafeUrl: (url: URL, context: string) => Promise<void> }) => {
  return tool(
    async ({ url, method = 'GET', headers, body }) => {
      try {
        const parsedUrl = new URL(url)
        await assertSafeUrl(parsedUrl, `curl tool fetch`)

        const __dirname = path.dirname(fileURLToPath(import.meta.url))
        const rustBin = path.join(__dirname, '../../../target/release/curl-tool')

        const args = [url, '--request', method]
        if (headers) {
          for (const [k, v] of Object.entries(headers)) {
            args.push('--header', `${k}: ${v}`)
          }
        }
        if (body) {
          args.push('--data', body)
        }

        args.push('--no-guard')

        const res = await execa(rustBin, args, { reject: false })
        if (res.failed) {
          return `Error: ${res.stderr || res.message}`
        }

        return res.stdout
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
