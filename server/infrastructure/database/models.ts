import { useDb } from './connection'
import { forbidden, notFound, internal } from '#server/core/errors/http'
import { and, eq } from 'drizzle-orm'
import { models, modelProviders } from '../../database/schema'

export interface ModelFields {
  modelId?: string
  label?: string
  description?: string
  icon?: string
  contextWindow?: number
  maxOutputTokens?: number
  thinkingEnabled?: boolean
  thinkingMinTokens?: number
  thinkingMaxTokens?: number
}

export async function listUserModels(userId: string) {
  return useDb().select().from(models).where(eq(models.userId, userId))
}

/**
 * Narrow ownership-resolution lookup: returns the model only when it both
 * exists and is owned by `userId`. Never distinguishes "doesn't exist" from
 * "belongs to someone else" — both surface as the same 404 so cross-tenant
 * probing can't be used to enumerate other users' model IDs.
 */
export async function findUserModel(userId: string, id: string) {
  const [model] = await useDb().select().from(models).where(and(eq(models.id, id), eq(models.userId, userId))).limit(1)
  if (!model) throw notFound('Model not found')
  return model
}

export async function insertUserModel(userId: string, providerId: string, body: ModelFields & { modelId: string, label: string }) {
  const db = useDb()
  const [provider] = await db.select().from(modelProviders).where(and(eq(modelProviders.id, providerId), eq(modelProviders.userId, userId)))
  if (!provider) throw forbidden('Provider not found or not owned by user')
  const [model] = await db.insert(models).values({ userId, providerId, ...body }).returning()
  if (!model) throw internal('Failed to create model')
  return model
}

export async function updateUserModel(userId: string, id: string, updates: ModelFields) {
  const updateData: ModelFields & { updatedAt: Date } = { updatedAt: new Date() }
  for (const field of ['modelId', 'label', 'description', 'icon', 'contextWindow', 'maxOutputTokens', 'thinkingEnabled', 'thinkingMinTokens', 'thinkingMaxTokens'] as const) {
    if (updates[field] !== undefined) updateData[field] = updates[field] as never
  }
  const [updated] = await useDb().update(models).set(updateData).where(and(eq(models.id, id), eq(models.userId, userId))).returning()
  if (!updated) throw notFound('Model not found')
  return updated
}

export async function deleteUserModel(userId: string, id: string) {
  const [deleted] = await useDb().delete(models).where(and(eq(models.id, id), eq(models.userId, userId))).returning()
  if (!deleted) throw notFound('Model not found')
  return { ok: true }
}
