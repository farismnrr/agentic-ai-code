import * as v from 'valibot'
import { providerRequiresBaseUrl } from '#shared/utils/providers'

const bodySchema = v.pipe(
  v.object({
    type: v.picklist(['openai_compatible', 'anthropic_compatible', 'vertex_ai']),
    name: v.string(),
    baseUrl: v.optional(v.string()),
    apiKey: v.string(),
    customHeaders: v.optional(v.record(v.string(), v.string()))
  }),
  // OpenAI/Anthropic-compatible services are self-hosted or third-party, so
  // there's no sensible default base URL to fall back to — an empty one
  // means every chat request and live model-list fetch fails later with an
  // opaque error instead of being rejected up front where the user is looking.
  v.forward(
    v.partialCheck(
      [['type'], ['baseUrl']],
      input => !providerRequiresBaseUrl(input.type) || !!input.baseUrl,
      'Base URL is required for this provider type'
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
