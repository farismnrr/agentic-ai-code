import { providerRequiresBaseUrl } from '#shared/utils/providers'
import { deleteUserProvider, findUserProvider, insertUserProvider, listUserProviders, updateUserProvider, type ProviderInput, type ProviderUpdate } from '../infrastructure/database/providers'
import { encryptSecret } from '../infrastructure/security/crypto'
import { badRequest, badGateway } from '../utils/http-errors'

export type CreateProviderBody = Omit<ProviderInput, 'customHeaders'> & { customHeaders?: Record<string, string> }
export type UpdateProviderBody = ProviderUpdate

export async function listModelProviders(userId: string) {
  return listUserProviders(userId)
}

export async function createModelProvider(userId: string, body: CreateProviderBody) {
  const apiKeyEncrypted = encryptSecret(body.apiKey)
  // New provider — every submitted header is a plaintext value to encrypt,
  // there's nothing to merge with yet.
  const customHeaders = Object.fromEntries(
    Object.entries(body.customHeaders ?? {}).map(([key, value]) => [key, encryptSecret(value)])
  )
  return insertUserProvider(userId, { ...body, customHeaders }, apiKeyEncrypted)
}

export async function updateModelProvider(userId: string, id: string, updates: UpdateProviderBody) {
  // `customHeaders` is a diff (see `ProviderUpdate`): encrypt the plaintext
  // values the caller is setting/replacing and pass `null` deletions
  // through untouched; the infrastructure layer merges this into the
  // stored (already-encrypted) header map.
  const customHeaders = updates.customHeaders === undefined
    ? undefined
    : Object.fromEntries(
        Object.entries(updates.customHeaders).map(([key, value]) => [key, value === null ? null : encryptSecret(value)])
      )
  return updateUserProvider(userId, id, { ...updates, customHeaders }, updates.apiKey ? encryptSecret(updates.apiKey) : undefined)
}

export async function deleteModelProvider(userId: string, id: string) {
  return deleteUserProvider(userId, id)
}

export async function listProviderModelIds(userId: string, providerId: string) {
  const provider = await findUserProvider(userId, providerId)
  if (providerRequiresBaseUrl(provider.type) && !provider.baseUrl) {
    throw badRequest(`${provider.name} has no base URL set`)
  }
  const { listProviderModels } = await import('../infrastructure/ai/providers/index')
  try {
    return await listProviderModels(provider)
  } catch (err) {
    throw badGateway(`Could not reach ${provider.name}: ${(err as Error).message}`)
  }
}
