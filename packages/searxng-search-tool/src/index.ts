import { tool } from '@langchain/core/tools'
import { z } from 'zod'
import { execa } from 'execa'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export type AiToolsEnvProvider = () => Record<string, string>

export const createSearxngSearchTool = ({ baseUrl, getChildEnv = () => ({}) }: { baseUrl: string, getChildEnv?: AiToolsEnvProvider }) => {
  return tool(
    async ({ query }) => {
      try {
        const __dirname = path.dirname(fileURLToPath(import.meta.url))
        const rustBin = path.join(__dirname, '../../../target/release/ai-tools')

        const cliArgs = ['searxng', query, '--base-url', baseUrl]
        const res = await execa(rustBin, cliArgs, {
          reject: false,
          extendEnv: false,
          env: getChildEnv()
        })
        if (res.failed || res.stdout.startsWith('Error:') || res.stdout.startsWith('Search failed')) return 'Tool execution failed'

        return res.stdout
      } catch {
        return 'Tool execution failed'
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
