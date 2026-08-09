import { eq, and } from 'drizzle-orm'
import { modelProviders, type ModelProviderType } from '../database/schema'
import { encryptSecret } from './crypto'
import { notFound, badRequest, badGateway } from './http-errors'
import { listProviderModels } from './providers/index'

const PROVIDER_TYPES_REQUIRING_BASE_URL: ModelProviderType[] = ['openai_compatible', 'anthropic_compatible']

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
    customHeaders: p.customHeaders,
    enabled: p.enabled,
    hasApiKey: !!p.apiKeyEncrypted
  }))
}

export async function createModelProvider(userId: string, body: { type: ModelProviderType, name: string, baseUrl?: string, apiKey: string, customHeaders?: Record<string, string> }) {
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
      customHeaders: body.customHeaders ?? {},
      enabled: true
    })
    .returning()

  if (!provider) throw internal('Failed to create model provider')

  return {
    id: provider.id,
    type: provider.type,
    name: provider.name,
    baseUrl: provider.baseUrl,
    customHeaders: provider.customHeaders,
    enabled: provider.enabled,
    hasApiKey: true
  }
}

export async function updateModelProvider(userId: string, id: string, updates: { name?: string, baseUrl?: string, apiKey?: string, customHeaders?: Record<string, string>, enabled?: boolean }) {
  const db = useDb()

  const [existing] = await db
    .select()
    .from(modelProviders)
    .where(and(eq(modelProviders.id, id), eq(modelProviders.userId, userId)))
    .limit(1)

  if (!existing) throw notFound('Provider not found')

  const nextBaseUrl = updates.baseUrl !== undefined ? updates.baseUrl : existing.baseUrl
  if (PROVIDER_TYPES_REQUIRING_BASE_URL.includes(existing.type) && !nextBaseUrl) {
    throw badRequest('Base URL is required for this provider type')
  }

  const updateData: any = {
    updatedAt: new Date()
  }

  if (updates.name !== undefined) updateData.name = updates.name
  if (updates.baseUrl !== undefined) updateData.baseUrl = updates.baseUrl
  if (updates.customHeaders !== undefined) updateData.customHeaders = updates.customHeaders
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
    customHeaders: updated.customHeaders,
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

export async function listProviderModelIds(userId: string, providerId: string) {
  const db = useDb()
  const [provider] = await db
    .select()
    .from(modelProviders)
    .where(and(eq(modelProviders.id, providerId), eq(modelProviders.userId, userId)))
    .limit(1)

  if (!provider) throw notFound('Provider not found')
  if (PROVIDER_TYPES_REQUIRING_BASE_URL.includes(provider.type) && !provider.baseUrl) {
    throw badRequest(`${provider.name} has no base URL set — edit the provider and add one before listing models`)
  }
  // Vertex AI Express Mode has no discovery endpoint at all — that's not a
  // reachability failure, so it shouldn't read as one (502).
  if (provider.type === 'vertex_ai') {
    throw badRequest('Vertex AI Express Mode has no model-listing endpoint — enter the model ID directly (e.g. gemini-2.5-flash, gemini-2.5-pro)')
  }

  try {
    return await listProviderModels(provider)
  } catch (err) {
    throw badGateway(`Could not reach ${provider.name}: ${(err as Error).message}`)
  }
}
