import { createCurlTool } from '@ai-code/curl-tool'
import { createSearxngSearchTool } from '@ai-code/searxng-search-tool'
import { assertSafeUrl } from './ssrf-guard'

export const langgraphTools = [
  createCurlTool({ assertSafeUrl }),
  createSearxngSearchTool({ baseUrl: useRuntimeConfig().searxngBaseUrl })
]
