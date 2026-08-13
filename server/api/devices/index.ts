import { badRequest, unprocessable } from '#server/core/errors/http'
import * as v from 'valibot'

const registerDeviceSchema = v.object({
  name: v.pipe(v.string(), v.minLength(1, 'Device name is required')),
  fingerprint: v.pipe(v.string(), v.minLength(1, 'Fingerprint is required'))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const userId = session.user.id
  const method = event.method

  if (method === 'GET') {
    return event.context.application.account.listUserDevices(userId)
  }

  if (method === 'POST') {
    const result = v.safeParse(registerDeviceSchema, await readBody(event))
    if (!result.success) throw unprocessable(result.issues)
    const { name, fingerprint } = result.output

    return event.context.application.account.registerUserDevice({ userId, name, fingerprint })
  }

  throw badRequest(`Unsupported method: ${method}`)
})
