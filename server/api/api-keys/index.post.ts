import * as v from 'valibot'
import { createApiKey } from '../../application/account-data'
import { generateApiKey } from '../../infrastructure/auth/api-key'

const createSchema = v.object({
  name: v.pipe(v.string(), v.minLength(1), v.maxLength(255))
})

export default defineEventHandler(async (event) => {
  const { user } = await requireUserSession(event)
  const body = await readValidatedBody(event, body => v.parse(createSchema, body))

  const { rawKey, keyPrefix, keyHash } = generateApiKey()
  const [created] = await createApiKey({ userId: user.id, name: body.name, keyHash, keyPrefix })

  return {
    ...created,
    rawKey // Only returned once upon creation!
  }
})
