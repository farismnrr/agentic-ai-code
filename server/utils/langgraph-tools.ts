import { createCurlTool } from '@ai-code/curl-tool'
import { createSearxngSearchTool } from '@ai-code/searxng-search-tool'
import { createTerminalTool } from '@ai-code/terminal-tool'
import { assertSafeUrl } from './ssrf-guard'
import { assertSafeCommand } from './exec-guard'

export const buildLanggraphTools = ({ workspacePath }: { workspacePath?: string }) => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const tools: any[] = [
    createCurlTool({ assertSafeUrl }),
    createSearxngSearchTool({ baseUrl: useRuntimeConfig().searxngBaseUrl })
  ]

  if (workspacePath) {
    tools.push(
      createTerminalTool({
        cwd: workspacePath,
        assertSafeCommand: (c, a) => assertSafeCommand(c, a, 'read-only')
      })
    )
  }

  return tools
}
