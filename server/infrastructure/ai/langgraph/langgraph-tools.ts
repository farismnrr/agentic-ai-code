import { createCurlTool } from '@ai-code/curl-tool'
import { createSearxngSearchTool } from '@ai-code/searxng-search-tool'
import { assertSafeUrl } from '../../security/ssrf-guard'
import { aiToolsTraceEnv } from '../../observability/ai-tools-trace'

export const buildLanggraphTools = () => [
  createCurlTool({ assertSafeUrl, getChildEnv: aiToolsTraceEnv }),
  createSearxngSearchTool({ baseUrl: useRuntimeConfig().searxngBaseUrl, getChildEnv: aiToolsTraceEnv })
]
