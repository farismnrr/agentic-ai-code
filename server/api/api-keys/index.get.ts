import { listApiKeys } from '../../infrastructure/composition'

export default defineEventHandler(async (event) => {
  const { user } = await requireUserSession(event)
  return listApiKeys(user.id)
})
