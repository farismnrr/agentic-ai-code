import * as v from 'valibot'
import { apiKeys } from '../../database/schema'

const createSchema = v.object({
  name: v.pipe(v.string(), v.minLength(1), v.maxLength(255))
})

export default defineEventHandler(async (event) => {
  const { user } = await requireUserSession(event)
  const body = await readValidatedBody(event, body => v.parse(createSchema, body))

  const { rawKey, keyPrefix, keyHash } = generateApiKey()
  const db = useDb()

  const [created] = await db
    .insert(apiKeys)
    .values({
      userId: user.id,
      name: body.name,
      keyHash,
      keyPrefix
    })
    .returning({
      id: apiKeys.id,
      name: apiKeys.name,
      keyPrefix: apiKeys.keyPrefix,
      createdAt: apiKeys.createdAt
    })

  return {
    ...created,
    rawKey // Only returned once upon creation!
  }
})
