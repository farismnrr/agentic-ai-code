/* eslint-disable @stylistic/max-statements-per-line */
import { badRequest, notFound, unprocessable } from '#server/core/errors/http'
import { updateTaskLedger } from '../../../application/task-context-output'
import * as v from 'valibot'

const schema = v.object({ tasks: v.array(v.object({ id: v.string(), title: v.string(), status: v.picklist(['pending', 'in_progress', 'blocked', 'completed', 'cancelled']), depends_on: v.optional(v.array(v.string())), short_note: v.optional(v.string()) })) })
export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event); const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing conversation ID')
  const conversation = await event.context.application.conversations.find(session.user.id, id)
  if (!conversation) throw notFound('Conversation not found')
  const result = v.safeParse(schema, await readBody(event)); if (!result.success) throw unprocessable(result.issues)
  try { return updateTaskLedger({ userId: session.user.id, conversationId: id, sessionId: id, tasks: result.output.tasks }) } catch { throw unprocessable([]) }
})
