import * as v from 'valibot'

const schema = v.object({
  id: v.nullable(v.pipe(v.string(), v.uuid()))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readValidatedBody(event, body => v.parse(schema, body))

  await event.context.application.workspaces.setActive(session.user.id, body.id)

  return { success: true }
})
