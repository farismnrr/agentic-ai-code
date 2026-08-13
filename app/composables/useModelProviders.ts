export interface ModelProvider {
  id: string
  type: 'openai_compatible' | 'anthropic_compatible' | 'vertex_ai'
  name: string
  baseUrl: string | null
  customHeaders: Record<string, string>
  enabled: boolean
  hasApiKey: boolean
}

export interface ModelProviderTypeOption {
  label: string
  value: ModelProvider['type']
}

export function useModelProviders() {
  const providers = useState<ModelProvider[]>('model-providers', () => [])
  const types = useState<ModelProviderTypeOption[]>('model-provider-types', () => [])

  async function load() {
    const fetch = import.meta.server ? useRequestFetch() : $fetch
    providers.value = await fetch<ModelProvider[]>('/api/providers')
    types.value = await fetch<ModelProviderTypeOption[]>('/api/providers/types')
  }

  async function create(data: Partial<ModelProvider> & { apiKey: string }) {
    const newProvider = await $fetch<ModelProvider>('/api/providers', {
      method: 'POST',
      body: data
    })
    providers.value.push(newProvider)
    return newProvider
  }

  async function update(id: string, data: Partial<ModelProvider> & { apiKey?: string }) {
    const updatedProvider = await $fetch<ModelProvider>(`/api/providers/${id}`, {
      method: 'PUT',
      body: data
    })
    const index = providers.value.findIndex(p => p.id === id)
    if (index !== -1) providers.value[index] = updatedProvider
    return updatedProvider
  }

  async function remove(id: string) {
    await $fetch(`/api/providers/${id}`, { method: 'DELETE' })
    providers.value = providers.value.filter(p => p.id !== id)
  }

  function listModels(providerId: string) {
    return $fetch<{ label: string, value: string }[]>(`/api/providers/${providerId}/models`)
  }

  return { providers, types, load, create, update, remove, listModels }
}
