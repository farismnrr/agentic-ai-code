import { eq, and } from 'drizzle-orm'
import { modelProviders } from '../database/schema'
import { encryptSecret } from './crypto'
import { notFound } from './http-errors'

export async function listModelProviders(userId: string) {
  const db = useDb()
  const providers = await db
    .select()
    .from(modelProviders)
    .where(eq(modelProviders.userId, userId))

  return providers.map(p => ({
    id: p.id,
    type: p.type,
    name: p.name,
    baseUrl: p.baseUrl,
    enabled: p.enabled,
    hasApiKey: !!p.apiKeyEncrypted
  }))
}

export async function createModelProvider(userId: string, body: { type: '9router' | 'gcp_agent_platform', name: string, baseUrl?: string, apiKey: string }) {
  const db = useDb()
  const apiKeyEncrypted = encryptSecret(body.apiKey)

  const [provider] = await db
    .insert(modelProviders)
    .values({
      userId,
      type: body.type,
      name: body.name,
      baseUrl: body.baseUrl,
      apiKeyEncrypted,
      enabled: true
    })
    .returning()

  return {
    id: provider.id,
    type: provider.type,
    name: provider.name,
    baseUrl: provider.baseUrl,
    enabled: provider.enabled,
    hasApiKey: true
  }
}

export async function updateModelProvider(userId: string, id: string, updates: { name?: string, baseUrl?: string, apiKey?: string, enabled?: boolean }) {
  const db = useDb()
  const updateData: any = {
    updatedAt: new Date()
  }

  if (updates.name !== undefined) updateData.name = updates.name
  if (updates.baseUrl !== undefined) updateData.baseUrl = updates.baseUrl
  if (updates.enabled !== undefined) updateData.enabled = updates.enabled
  if (updates.apiKey) updateData.apiKeyEncrypted = encryptSecret(updates.apiKey)

  const [updated] = await db
    .update(modelProviders)
    .set(updateData)
    .where(and(eq(modelProviders.id, id), eq(modelProviders.userId, userId)))
    .returning()

  if (!updated) throw notFound('Provider not found')

  return {
    id: updated.id,
    type: updated.type,
    name: updated.name,
    baseUrl: updated.baseUrl,
    enabled: updated.enabled,
    hasApiKey: !!updated.apiKeyEncrypted
  }
}

export async function deleteModelProvider(userId: string, id: string) {
  const db = useDb()
  const [deleted] = await db
    .delete(modelProviders)
    .where(and(eq(modelProviders.id, id), eq(modelProviders.userId, userId)))
    .returning()

  if (!deleted) throw notFound('Provider not found')
  return { ok: true }
}
