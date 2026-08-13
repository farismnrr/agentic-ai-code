import type { ChatModel } from '#shared/types/chat'
import { removeById, replaceById } from '../utils/collection'

export type Model = ChatModel

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
    if (models.value.some(model => model.id === id)) models.value = replaceById(models.value, id, updatedModel)
    return updatedModel
  }

  async function remove(id: string) {
    await $fetch(`/api/models/${id}`, { method: 'DELETE' })
    models.value = removeById(models.value, id)
  }

  return { models, load, create, update, remove }
}
