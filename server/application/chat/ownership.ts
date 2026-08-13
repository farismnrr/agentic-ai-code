import { badRequest, notFound } from '../../utils/http-errors'
import type { ChatOwnershipPort } from './contracts'

/**
 * The one authoritative same-tenant rule for "does this model belong to
 * this user, and does the provider backing it also belong to this user".
 *
 * Every place that persists or consumes a client-supplied `modelId` for a
 * user's conversation/settings (create, update, chat-turn context load,
 * `defaultModelId` settings updates) must route through this function
 * instead of loading the model/provider by ID directly. UI-side filtering
 * of the model picker is not authorization — this is the server-side check
 * Plan 031A Finding A requires.
 *
 * Fails closed: a model that exists but belongs to another user, a model
 * whose provider belongs to another user (should be impossible via normal
 * writes, but existing rows are not trusted), or a missing model/provider
 * all surface as the same non-enumerating 404 from the underlying narrow
 * lookups.
 */
export async function resolveOwnedModelContext(userId: string, modelId: string, ownership?: ChatOwnershipPort) {
  if (!ownership) throw new Error('Chat ownership port is required')
  const model = await ownership.findModel(userId, modelId)
  const provider = await ownership.findProvider(userId, model.providerId)
  return { model, provider }
}

/**
 * The one authoritative same-tenant rule for "does this workspace belong to
 * this user". Every place that persists or consumes a client-supplied
 * `workspaceId` for a user's conversation (create, any workspace-change
 * path, chat-turn workspace-context resolution) must route through this
 * function instead of loading the workspace by ID directly. Foreign keys
 * only prove the workspace exists, not that it belongs to the same tenant —
 * this is the server-side check Plan 031A Finding B requires.
 */
export async function resolveOwnedWorkspace(userId: string, workspaceId: string, ownership?: ChatOwnershipPort) {
  if (!ownership) throw new Error('Chat ownership port is required')
  return ownership.findWorkspace(userId, workspaceId)
}

/**
 * The authoritative context load for a chat turn: authorizes the
 * conversation itself, then reasserts conversation -> model -> provider
 * same-user ownership via `resolveOwnedModelContext` rather than trusting
 * the conversation's stored `modelId` (Plan 031A finding A). Fails closed
 * with a 404 if the conversation, model, or provider don't exist or don't
 * all belong to `userId` — including for rows written before this fix that
 * might otherwise reference a model/provider not owned by the same user.
 */
export async function loadAuthorizedChatContext(userId: string, conversationId: string, ownership: ChatOwnershipPort) {
  if (!conversationId) throw badRequest('Missing conversationId')
  const conversation = await ownership.findConversation(userId, conversationId)
  if (!conversation) throw notFound('Conversation not found')
  const { model, provider } = await resolveOwnedModelContext(userId, conversation.modelId, ownership)
  return { conversation, model, provider }
}
