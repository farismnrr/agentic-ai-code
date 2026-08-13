import { deleteApiKey } from '../../infrastructure/composition'
import * as v from 'valibot'

const idSchema = v.object({
  id: v.pipe(v.string(), v.uuid())
})

export default defineEventHandler(async (event) => {
  const { user } = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  const params = v.parse(idSchema, { id })

  const [deleted] = await deleteApiKey(user.id, params.id)

  if (!deleted) {
    throw createError({
      statusCode: 404,
      message: 'API Key not found'
    })
  }

  return deleted
})
