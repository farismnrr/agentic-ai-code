import { setActiveWorkspace } from '../../application/account-data'
import { findUserWorkspace } from '../../infrastructure/database/workspaces'
import * as v from 'valibot'

const schema = v.object({
  id: v.nullable(v.pipe(v.string(), v.uuid()))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readValidatedBody(event, body => v.parse(schema, body))

  if (body.id !== null) {
    await findUserWorkspace(session.user.id, body.id)
  }

  await setActiveWorkspace(session.user.id, body.id)

  return { success: true }
})
