import * as v from 'valibot'

const paramsSchema = v.object({
  id: v.pipe(v.string(), v.uuid('Invalid device ID format'))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const userId = session.user.id

  const result = v.safeParse(paramsSchema, { id: getRouterParam(event, 'id') })
  if (!result.success) throw unprocessable(result.issues)

  const [device] = await event.context.application.account.revokeDevice(userId, result.output.id)

  if (!device) {
    throw notFound('Device not found')
  }

  return { success: true, device }
})
