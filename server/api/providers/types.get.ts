import { PROVIDER_TYPE_OPTIONS } from '#shared/utils/providers'

export default defineEventHandler(async (event) => {
  await requireUserSession(event)
  return PROVIDER_TYPE_OPTIONS
})
