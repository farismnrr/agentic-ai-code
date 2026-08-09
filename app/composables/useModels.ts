export interface Model {
  id: string
  providerId: string
  modelId: string
  name: string
  label: string
  description: string
  icon: string
  contextWindow: number | null
  maxOutputTokens: number | null
  thinkingEnabled: boolean | null
  thinkingMinTokens: number | null
  thinkingMaxTokens: number | null
}

export function useModels() {
  const models = useState<Model[]>('models', () => [])

  async function load() {
    const fetch = import.meta.server ? useRequestFetch() : $fetch
    models.value = await fetch<Model[]>('/api/models')
  }

  async function create(data: Partial<Model>) {
    const newModel = await $fetch<Model>('/api/models', {
      method: 'POST',
      body: data
    })
    models.value.push(newModel)
    return newModel
  }

  async function update(id: string, data: Partial<Model>) {
    const updatedModel = await $fetch<Model>(`/api/models/${id}`, {
      method: 'PUT',
      body: data
    })
    const index = models.value.findIndex(m => m.id === id)
    if (index !== -1) models.value[index] = updatedModel
    return updatedModel
  }

  async function remove(id: string) {
    await $fetch(`/api/models/${id}`, { method: 'DELETE' })
    models.value = models.value.filter(m => m.id !== id)
  }

  return { models, load, create, update, remove }
}
