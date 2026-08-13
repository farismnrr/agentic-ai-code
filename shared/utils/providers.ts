import type { ModelProvider } from '#shared/types/chat'

export const PROVIDER_TYPES_REQUIRING_BASE_URL: ModelProvider['type'][] = [
  'openai_compatible',
  'anthropic_compatible'
]

export const PROVIDER_TYPE_OPTIONS = [
  { label: 'OpenAI Compatible', value: 'openai_compatible' },
  { label: 'Anthropic Compatible', value: 'anthropic_compatible' },
  { label: 'Vertex AI', value: 'vertex_ai' }
] satisfies Array<{ label: string, value: ModelProvider['type'] }>

export function providerRequiresBaseUrl(type: ModelProvider['type']) {
  return PROVIDER_TYPES_REQUIRING_BASE_URL.includes(type)
}
