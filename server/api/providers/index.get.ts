import { listModelProviders } from '../../infrastructure/composition'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  return listModelProviders(session.user.id)
})
