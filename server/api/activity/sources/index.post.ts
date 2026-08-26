import * as v from 'valibot'
import { badRequest } from '#server/core/errors/http'

const safeText = (max: number, min = 0) => v.pipe(v.string(), v.minLength(min), v.maxLength(max), v.check(value => ![...value].some(character => character <= '\u001f' || character === '\u007f')))

const schema = v.strictObject({
  label: safeText(80, 1),
  kind: safeText(32, 1),
  deviceId: v.optional(v.pipe(v.string(), v.uuid()))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const parsed = v.safeParse(schema, await readBody(event))
  if (!parsed.success) throw badRequest('Activity source details are invalid')
  setResponseHeader(event, 'Cache-Control', 'private, no-store')
  // The returned token is intentionally present only in this response. The
  // database stores only its hash and prefix.
  return event.context.application.activity.enroll(session.user.id, parsed.output)
})
