import * as v from 'valibot'

const bodySchema = v.pipe(
  v.object({
    type: v.picklist(['9router', 'gcp_agent_platform']),
    name: v.string(),
    baseUrl: v.optional(v.string()),
    apiKey: v.string()
  }),
  // 9Router speaks a self-hosted OpenAI-compatible API, so there's no
  // sensible default base URL to fall back to — an empty one means every
  // chat request and live model-list fetch fails later with an opaque
  // error instead of being rejected up front where the user is looking.
  v.forward(
    v.partialCheck(
      [['type'], ['baseUrl']],
      input => input.type !== '9router' || !!input.baseUrl,
      'Base URL is required for 9Router providers'
    ),
    ['baseUrl']
  )
)

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readBody(event)
  const parsed = v.safeParse(bodySchema, body)
  if (!parsed.success) {
    throw unprocessable(parsed.issues)
  }
  return createModelProvider(session.user.id, parsed.output)
})
