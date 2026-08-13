import { badRequest, unprocessable } from '#server/core/errors/http'
import * as v from 'valibot'

const updateSchema = v.object({
  name: v.string(),
  path: v.optional(v.string())
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing workspace id')

  const result = v.safeParse(updateSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  return event.context.application.workspaces.update(session.user.id, id, body.name, body.path)
})
