import { eq, and } from 'drizzle-orm'
import { models, modelProviders } from '../database/schema'
import { notFound, forbidden } from './http-errors'

export async function listModels(userId: string) {
  const db = useDb()
  const userModels = await db
    .select()
    .from(models)
    .where(eq(models.userId, userId))
  return userModels
}

export async function createModel(userId: string, providerId: string, body: any) {
  const db = useDb()
  // Ensure provider exists and belongs to user
  const [provider] = await db
    .select()
    .from(modelProviders)
    .where(and(eq(modelProviders.id, providerId), eq(modelProviders.userId, userId)))

  if (!provider) throw forbidden('Provider not found or not owned by user')

  const [model] = await db
    .insert(models)
    .values({
      userId,
      providerId,
      modelId: body.modelId,
      label: body.label,
      description: body.description,
      icon: body.icon,
      contextWindow: body.contextWindow,
      maxOutputTokens: body.maxOutputTokens,
      thinkingEnabled: body.thinkingEnabled,
      thinkingMinTokens: body.thinkingMinTokens,
      thinkingMaxTokens: body.thinkingMaxTokens
    })
    .returning()

  if (!model) throw internal('Failed to create model')
  return model
}

export async function updateModel(userId: string, id: string, updates: any) {
  const db = useDb()

  const updateData: any = {
    updatedAt: new Date()
  }
  const fields = ['modelId', 'label', 'description', 'icon', 'contextWindow', 'maxOutputTokens', 'thinkingEnabled', 'thinkingMinTokens', 'thinkingMaxTokens']
  for (const field of fields) {
    if (updates[field] !== undefined) {
      updateData[field] = updates[field]
    }
  }

  const [updated] = await db
    .update(models)
    .set(updateData)
    .where(and(eq(models.id, id), eq(models.userId, userId)))
    .returning()

  if (!updated) throw notFound('Model not found')
  return updated
}

export async function deleteModel(userId: string, id: string) {
  const db = useDb()
  const [deleted] = await db
    .delete(models)
    .where(and(eq(models.id, id), eq(models.userId, userId)))
    .returning()

  if (!deleted) throw notFound('Model not found')
  return { ok: true }
}
