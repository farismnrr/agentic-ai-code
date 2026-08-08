import * as v from 'valibot'

const createSchema = v.object({
  name: v.string(),
  path: v.string()
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)

  const result = v.safeParse(createSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  return createWorkspace(session.user.id, body.name, body.path)
})
