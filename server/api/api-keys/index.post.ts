import * as v from 'valibot'

const createSchema = v.object({
  name: v.pipe(v.string(), v.minLength(1), v.maxLength(255))
})

export default defineEventHandler(async (event) => {
  const { user } = await requireUserSession(event)
  const body = await readValidatedBody(event, body => v.parse(createSchema, body))

  const { rawKey, keyPrefix, keyHash } = event.context.application.account.generateApiKey()
  const [created] = await event.context.application.account.createApiKey({ userId: user.id, name: body.name, keyHash, keyPrefix })

  return {
    ...created,
    rawKey // Only returned once upon creation!
  }
})
