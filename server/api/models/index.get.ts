import { listModels } from '../../infrastructure/composition'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return listModels(session.user.id)
})
