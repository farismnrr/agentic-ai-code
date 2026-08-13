import { providerRequiresBaseUrl } from '#shared/utils/providers'
import { deleteUserProvider, findUserProvider, insertUserProvider, listUserProviders, updateUserProvider, type ProviderInput, type ProviderUpdate } from '../database/providers'
import { encryptSecret } from '../security/crypto'
import { badRequest, badGateway } from '../../utils/http-errors'
import * as accountData from '../database/account-data'
import * as auth from '../database/auth'
import * as features from '../database/settings'
import * as workspaces from '../database/workspaces'
import * as mcpServers from '../database/mcp-servers'
import * as messages from '../database/messages'
import * as models from '../../utils/models'
import { generateApiKey, verifyApiKey } from '../auth/api-key'
import { createChatTurnDependencies } from '../ai/chat-turn-dependencies'

export * from '../database/account-data'
export * from '../database/auth'
export * from '../database/settings'
export * from '../database/workspaces'
export * from '../database/mcp-servers'
export * from '../database/messages'
export * from '../../utils/models'
export { generateApiKey, verifyApiKey, createChatTurnDependencies }

export async function listModelProviders(userId: string) {
  return listUserProviders(userId)
}
export type CreateProviderBody = Omit<ProviderInput, 'customHeaders'> & { customHeaders?: Record<string, string> }
export type UpdateProviderBody = ProviderUpdate
export async function createModelProvider(userId: string, body: CreateProviderBody) {
  const customHeaders = Object.fromEntries(Object.entries(body.customHeaders ?? {}).map(([key, value]) => [key, encryptSecret(value)]))
  return insertUserProvider(userId, { ...body, customHeaders }, encryptSecret(body.apiKey))
}
export async function updateModelProvider(userId: string, id: string, updates: UpdateProviderBody) {
  const customHeaders = updates.customHeaders === undefined ? undefined : Object.fromEntries(Object.entries(updates.customHeaders).map(([key, value]) => [key, value === null ? null : encryptSecret(value)]))
  return updateUserProvider(userId, id, { ...updates, customHeaders }, updates.apiKey ? encryptSecret(updates.apiKey) : undefined)
}
export const deleteModelProvider = deleteUserProvider
export async function listProviderModelIds(userId: string, providerId: string) {
  const provider = await findUserProvider(userId, providerId)
  if (providerRequiresBaseUrl(provider.type) && !provider.baseUrl) throw badRequest(`${provider.name} has no base URL set`)
  try {
    const { listProviderModels } = await import('../ai/providers/index')
    return await listProviderModels(provider)
  } catch (err) { throw badGateway(`Could not reach ${provider.name}: ${(err as Error).message}`) }
}
export const getSettings = features.getSettings
export const updateSettings = features.updateSettings
export const listWorkspaces = workspaces.listWorkspaces
export const createWorkspace = workspaces.createWorkspace
export const updateWorkspace = workspaces.updateWorkspace
export const deleteWorkspace = workspaces.deleteWorkspace
export const findUserWorkspace = workspaces.findUserWorkspace
export const listMcpServers = mcpServers.listMcpServers
export const createMcpServer = mcpServers.createMcpServer
export const updateMcpServer = mcpServers.updateMcpServer
export const deleteMcpServer = mcpServers.deleteMcpServer
export const listConversationMessages = messages.listConversationMessages
export const sendMessage = messages.sendMessage
export const listModels = models.listModels
export const createModel = models.createModel
export const updateModel = models.updateModel
export const deleteModel = models.deleteModel
export const addVerificationToken = auth.addVerificationToken
export const consumeEmailVerification = auth.consumeEmailVerification
export const consumePasswordReset = auth.consumePasswordReset
export const createUser = auth.createUser
export const findLoginUser = auth.findLoginUser
export const findUserByEmail = auth.findUserByEmail
export const userExists = auth.userExists
export const listConversationSummaries = accountData.listConversationSummaries
export const listSidebarData = accountData.listSidebarData
export const setActiveWorkspace = accountData.setActiveWorkspace
export const listUserDevices = accountData.listUserDevices
export const registerUserDevice = accountData.registerUserDevice
export const listApiKeys = accountData.listApiKeys
export const createApiKey = accountData.createApiKey
export const deleteApiKey = accountData.deleteApiKey
export const createConversation = accountData.createConversation
export const updateConversation = accountData.updateConversation
export const deleteConversation = accountData.deleteConversation
export const revokeDevice = accountData.revokeDevice

export async function testMcpServer(userId: string, id: string) {
  const { testMcpServer } = await import('../mcp/test-server')
  return testMcpServer(userId, id)
}
