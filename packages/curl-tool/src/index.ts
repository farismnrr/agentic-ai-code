import { tool } from '@langchain/core/tools'
import { z } from 'zod'
import { execa } from 'execa'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export type AiToolsEnvProvider = () => Record<string, string>

export const createCurlTool = ({ assertSafeUrl, getChildEnv = () => ({}) }: { assertSafeUrl: (url: URL, context: string) => Promise<void>, getChildEnv?: AiToolsEnvProvider }) => {
  return tool(
    async ({ url, method = 'GET', headers, body }) => {
      try {
        const parsedUrl = new URL(url)
        await assertSafeUrl(parsedUrl, `curl tool fetch`)

        const __dirname = path.dirname(fileURLToPath(import.meta.url))
        const rustBin = path.join(__dirname, '../../../target/release/ai-tools')

        const args = [
          'curl',
          url,
          '--request', method,
          '--timeout', '30000',
          '--no-guard'
        ]

        if (headers) {
          for (const [k, v] of Object.entries(headers)) {
            args.push('--header', `${k}: ${v}`)
          }
        }
        if (body) {
          args.push('--data', body)
        }

        args.push('--no-guard')

        const res = await execa(rustBin, args, { reject: false, extendEnv: false, env: getChildEnv() })
        if (res.failed || res.stdout.startsWith('Error:')) return 'Tool execution failed'

        return res.stdout
      } catch {
        return 'Tool execution failed'
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
