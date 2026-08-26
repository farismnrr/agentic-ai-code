import * as v from 'valibot'
import { badRequest } from '#server/core/errors/http'

const schema = v.strictObject({
  sourceId: v.pipe(v.string(), v.minLength(1), v.maxLength(64)),
  workspaceId: v.pipe(v.string(), v.minLength(1), v.maxLength(64))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const parsed = v.safeParse(schema, await readBody(event))
  if (!parsed.success) throw badRequest('Activity workspace binding is invalid')
  await event.context.application.activity.bind(session.user.id, parsed.output.sourceId, parsed.output.workspaceId)
  return { bound: true }
})
