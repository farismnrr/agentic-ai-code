import { listConversationMessages } from '../../infrastructure/composition'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing conversation ID')

  return listConversationMessages(session.user.id, id)
})
