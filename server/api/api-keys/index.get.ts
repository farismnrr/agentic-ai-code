import { listApiKeys } from '../../application/account-data'

export default defineEventHandler(async (event) => {
  const { user } = await requireUserSession(event)
  return listApiKeys(user.id)
})
