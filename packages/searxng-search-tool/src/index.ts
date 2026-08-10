// @ts-ignore - Ignore module resolution errors in CI
import { tool } from '@langchain/core/tools'
import { z } from 'zod'
import { execa } from 'execa'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export const createSearxngSearchTool = ({ baseUrl }: { baseUrl: string }) => {
  return tool(
    async ({ query }) => {
      try {
        const __dirname = path.dirname(fileURLToPath(import.meta.url))
        const rustBin = path.join(__dirname, '../../../target/release/searxng-search-tool')

        const args = [query, '--base-url', baseUrl]

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
      name: 'searxng_search',
      description: 'Search the web using SearxNG.',
      schema: z.object({
        query: z.string().describe('The search query.')
      })
    }
  )
}
