import { and, eq } from 'drizzle-orm'
import { apiKeys } from '../../database/schema'
import * as v from 'valibot'

const idSchema = v.object({
  id: v.pipe(v.string(), v.uuid())
})

export default defineEventHandler(async (event) => {
  const { user } = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  const params = v.parse(idSchema, { id })

  const db = useDb()
  const [deleted] = await db
    .delete(apiKeys)
    .where(
      and(
        eq(apiKeys.id, params.id),
        eq(apiKeys.userId, user.id)
      )
    )
    .returning({ id: apiKeys.id })

  if (!deleted) {
    throw createError({
      statusCode: 404,
      message: 'API Key not found'
    })
  }

  return deleted
})
