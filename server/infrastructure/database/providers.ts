import { useDb } from './connection'
import { badRequest, notFound, internal } from '#server/core/errors/http'
import { and, eq } from 'drizzle-orm'
import { modelProviders, type ModelProviderType } from '../../database/schema'

import { encryptSecret, isEncryptedSecret } from '../security/crypto'
import { providerRequiresBaseUrl } from '#shared/utils/providers'

// `customHeaders` values are encrypted ciphertext by the time they reach this
// module (see `server/application/provider-management.ts`) — this layer only stores/merges
// them, it never sees or logs plaintext secret header values.
export type ProviderInput = { type: ModelProviderType, name: string, baseUrl?: string, apiKey: string, customHeaders?: Record<string, string> }
// `customHeaders` here is a diff: a string value replaces/adds that header
// (already encrypted), `null` deletes it, and a key simply absent from the
// object leaves the stored header untouched.
export type ProviderUpdate = { name?: string, baseUrl?: string, apiKey?: string, customHeaders?: Record<string, string | null>, enabled?: boolean }

// Client-visible projection never includes header values — only names, so
// the UI can show "this header is set" without ever round-tripping the
// secret (mirrors `hasApiKey` for the API key).
export function projectProvider(provider: typeof modelProviders.$inferSelect) {
  return { id: provider.id, type: provider.type, name: provider.name, baseUrl: provider.baseUrl, customHeaderKeys: Object.keys(provider.customHeaders), enabled: provider.enabled, hasApiKey: !!provider.apiKeyEncrypted }
}

export async function listUserProviders(userId: string) {
  const rows = await useDb().select().from(modelProviders).where(eq(modelProviders.userId, userId))
  return rows.map(projectProvider)
}

export async function findUserProvider(userId: string, id: string) {
  const [provider] = await useDb().select().from(modelProviders).where(and(eq(modelProviders.id, id), eq(modelProviders.userId, userId))).limit(1)
  if (!provider) throw notFound('Provider not found')
  return upgradeLegacyPlaintextHeaders(provider)
}

/**
 * Lazy upgrade for Plan 031A Finding R: rows written before `customHeaders`
 * values were encrypted may still hold legacy plaintext values. Every time
 * such a row is loaded for actual use (chat-turn ownership resolution,
 * update-flow lookups, provider discovery) this opportunistically
 * re-encrypts those values and persists them, so the row is upgraded in
 * place without a blind SQL backfill (which can't decrypt/re-encrypt
 * arbitrary plaintext at the DB layer).
 *
 * Idempotent by construction: a value already shaped like ciphertext
 * (`isEncryptedSecret`) is left untouched, so re-running this on an
 * already-upgraded or concurrently-upgraded row is a no-op — no
 * double-encryption risk.
 */
async function upgradeLegacyPlaintextHeaders(provider: typeof modelProviders.$inferSelect) {
  const entries = Object.entries(provider.customHeaders)
  if (entries.length === 0 || entries.every(([, value]) => isEncryptedSecret(value))) {
    return provider
  }
  const upgraded = Object.fromEntries(entries.map(([key, value]) => [key, isEncryptedSecret(value) ? value : encryptSecret(value)]))
  const [updated] = await useDb().update(modelProviders)
    .set({ customHeaders: upgraded, updatedAt: new Date() })
    .where(and(eq(modelProviders.id, provider.id), eq(modelProviders.userId, provider.userId)))
    .returning()
  return updated ?? { ...provider, customHeaders: upgraded }
}

export async function insertUserProvider(userId: string, input: ProviderInput, apiKeyEncrypted: string) {
  const [provider] = await useDb().insert(modelProviders).values({ userId, type: input.type, name: input.name, baseUrl: input.baseUrl, apiKeyEncrypted, customHeaders: input.customHeaders ?? {}, enabled: true }).returning()
  if (!provider) throw internal('Failed to create model provider')
  return projectProvider(provider)
}

export async function updateUserProvider(userId: string, id: string, updates: ProviderUpdate, apiKeyEncrypted?: string) {
  const existing = await findUserProvider(userId, id)
  const nextBaseUrl = updates.baseUrl !== undefined ? updates.baseUrl : existing.baseUrl
  if (providerRequiresBaseUrl(existing.type) && !nextBaseUrl) throw badRequest('Base URL is required for this provider type')
  const updateData: Record<string, unknown> = { updatedAt: new Date() }
  if (updates.name !== undefined) updateData.name = updates.name
  if (updates.baseUrl !== undefined) updateData.baseUrl = updates.baseUrl
  if (updates.customHeaders !== undefined) {
    const merged: Record<string, string> = {}
    for (const [key, value] of Object.entries(existing.customHeaders)) {
      if (updates.customHeaders[key] !== null) merged[key] = value
    }
    for (const [key, value] of Object.entries(updates.customHeaders)) {
      if (value !== null) merged[key] = value
    }
    updateData.customHeaders = merged
  }
  if (updates.enabled !== undefined) updateData.enabled = updates.enabled
  if (apiKeyEncrypted) updateData.apiKeyEncrypted = apiKeyEncrypted
  const [updated] = await useDb().update(modelProviders).set(updateData).where(and(eq(modelProviders.id, id), eq(modelProviders.userId, userId))).returning()
  if (!updated) throw notFound('Provider not found')
  return projectProvider(updated)
}

export async function deleteUserProvider(userId: string, id: string) {
  const [deleted] = await useDb().delete(modelProviders).where(and(eq(modelProviders.id, id), eq(modelProviders.userId, userId))).returning()
  if (!deleted) throw notFound('Provider not found')
  return { ok: true }
}
