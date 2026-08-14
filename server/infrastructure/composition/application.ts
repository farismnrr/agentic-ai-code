import { SpanStatusCode } from '@opentelemetry/api'
import { useDb } from '../database/connection'
import { getLogger, getTracer } from '../observability/otel'
import { resolveWorkspacePath } from '../filesystem/browse'
import { generateToken, hashToken } from '../security/token'
import { isUniqueViolation } from '../database/errors'
import { logger } from '../observability/logger'
import { createRequestTelemetryContext } from '../observability/request-context'
import { useMailer } from '../mail/mailer'
import { rateLimit } from '../network/rate-limit'
import { badRequest, badGateway } from '#server/core/errors/http'
import * as account from '../database/account-data'
import * as messages from '../database/messages'
import * as settings from '../database/settings'
import * as workspaces from '../database/workspaces'
import { findUserModel, listUserModels, insertUserModel, updateUserModel, deleteUserModel } from '../database/models'
import { resolveOwnedModelContext, resolveOwnedWorkspace } from '../../application/chat/ownership'
import { createConversationUseCases, type ConversationPort } from '../../application/conversations'
import { createSettingsUseCases } from '../../application/settings'
import { createProviderManagementUseCases, type ProviderManagementPort } from '../../application/provider-management'
import { createWorkspaceUseCases, type WorkspacePort } from '../../application/workspaces'
import { createModelUseCases, type ModelPort } from '../../application/models'
import { providerRequiresBaseUrl } from '#shared/utils/providers'

import { encryptSecret } from '../security/crypto'
import { insertUserProvider, listUserProviders, updateUserProvider, deleteUserProvider, findUserProvider, type ProviderInput, type ProviderUpdate } from '../database/providers'
import { listProviderModels } from '../ai/providers/index'
import { createChatTurnDependencies } from '../ai/chat-turn-dependencies'
import * as auth from '../database/auth'
import * as mcp from '../database/mcp-servers'
import * as apiKey from '../auth/api-key'
import type { AuthUseCases } from '../../application/auth'
import type { AccountDataUseCases } from '../../application/account-data'
import type { McpUseCases } from '../../application/mcp'

const conversationPort: ConversationPort = {
  list: account.listConversationSummaries as ConversationPort['list'],
  create: async input => (await account.createConversation(input as never))[0] as never,
  update: async (userId, id, input) => (await account.updateConversation(userId, id, input))[0] as never,
  remove: async (userId, id) => (await account.deleteConversation(userId, id))[0] as never,
  listMessages: messages.listConversationMessages,
  assertModelOwnership: async (userId, modelId) => { await resolveOwnedModelContext(userId, modelId, { findConversation: async () => undefined, findModel: (u, id) => findUserModel(u, id) as never, findProvider: (u, id) => findUserProvider(u, id) as never, findWorkspace: async () => undefined as never }) },
  assertWorkspaceOwnership: async (userId, workspaceId) => { await resolveOwnedWorkspace(userId, workspaceId, { findConversation: async () => undefined, findModel: (u, id) => findUserModel(u, id) as never, findProvider: (u, id) => findUserProvider(u, id) as never, findWorkspace: async (u, id) => (await import('../database/workspaces')).findUserWorkspace(u, id) as never }) }
}

export function createApplicationAdapters(requestId: string) {
  const request = createRequestTelemetryContext(requestId)
  return {
    chat: createChatTurnDependencies,

    network: { rateLimit },
    mail: useMailer(),
    observability: { logger, getLogger, request },
    database: { isUniqueViolation, db: useDb() },
    security: { generateToken, hashToken },
    filesystem: { resolveWorkspacePath },
    auth: { ...auth, verifyApiKey: apiKey.verifyApiKey } satisfies AuthUseCases,
    account: {
      listSidebarData: account.listSidebarData,
      generateApiKey: apiKey.generateApiKey,
      listUserDevices: account.listUserDevices,
      registerUserDevice: account.registerUserDevice,
      revokeDevice: account.revokeDevice,
      listApiKeys: account.listApiKeys,
      createApiKey: account.createApiKey,
      deleteApiKey: account.deleteApiKey
    } satisfies AccountDataUseCases,
    mcp: {
      testMcpServer: async (userId, id) => request.withSpan('mcp_server.test', {}, async () => (await import('../mcp/test-server')).testMcpServer(userId, id)),
      listServers: mcp.listMcpServers,
      createServer: async (userId: string, input: unknown) => request.withSpan('mcp_server.create', {}, () => mcp.createMcpServer(userId, input as never)),
      updateServer: async (userId: string, id: string, input: unknown) => request.withSpan('mcp_server.update', {}, () => mcp.updateMcpServer(userId, id, input as never)),
      deleteServer: async (userId: string, id: string) => request.withSpan('mcp_server.delete', {}, () => mcp.deleteMcpServer(userId, id)),
      listMessages: messages.listConversationMessages,
      sendMessage: messages.sendMessage
    } satisfies McpUseCases,
    conversations: createConversationUseCases(conversationPort, request),
    settings: createSettingsUseCases({ read: settings.getSettings, write: settings.updateSettings }, conversationPort.assertModelOwnership, request),
    providers: createProviderManagementUseCases(providerPort, request),
    workspaces: createWorkspaceUseCases(workspacePort, request),
    models: createModelUseCases(modelPort, request)
  }
}

const workspacePort: WorkspacePort<Awaited<ReturnType<typeof workspaces.listWorkspaces>>[number]> = {
  list: workspaces.listWorkspaces,
  create: workspaces.createWorkspace,
  update: workspaces.updateWorkspace,
  remove: workspaces.deleteWorkspace,
  find: workspaces.findUserWorkspace,
  setActive: account.setActiveWorkspace
}

type ModelInput = Parameters<typeof import('../database/models')['insertUserModel']>[2]
type ModelUpdate = Parameters<typeof import('../database/models')['updateUserModel']>[2]
const modelPort: ModelPort<Awaited<ReturnType<typeof findUserModel>>, ModelInput, ModelUpdate> = {
  list: listUserModels,
  create: insertUserModel,
  update: updateUserModel,
  remove: deleteUserModel
}

type ProviderCreate = Omit<ProviderInput, 'customHeaders'> & { customHeaders?: Record<string, string> }
const providerPort: ProviderManagementPort<ProviderCreate, ProviderUpdate, Awaited<ReturnType<typeof listUserProviders>>[number]> = {
  list: listUserProviders,
  create: async (userId, input) => insertUserProvider(userId, { ...input, customHeaders: Object.fromEntries(Object.entries(input.customHeaders ?? {}).map(([key, value]) => [key, encryptSecret(value)])) }, encryptSecret(input.apiKey)),
  update: async (userId, id, updates) => updateUserProvider(userId, id, { ...updates, customHeaders: updates.customHeaders && Object.fromEntries(Object.entries(updates.customHeaders).map(([key, value]) => [key, value === null ? null : encryptSecret(value)])) }, updates.apiKey ? encryptSecret(updates.apiKey) : undefined),
  remove: deleteUserProvider,
  discoverModels: async (userId, id) => {
    const provider = await findUserProvider(userId, id)
    if (providerRequiresBaseUrl(provider.type) && !provider.baseUrl) throw badRequest(`${provider.name} has no base URL set`)
    const tracer = getTracer('ai-code-server')
    return tracer.startActiveSpan('provider.reachability_check', { attributes: { 'provider.type': provider.type } }, async (span) => {
      try {
        const result = await listProviderModels(provider)
        span.end()
        return result
      } catch (error) {
        span.recordException(error instanceof Error ? error : String(error))
        span.setStatus({ code: SpanStatusCode.ERROR })
        span.end()
        throw badGateway(error, `Could not reach provider "${provider.name}"`)
      }
    })
  }
}
