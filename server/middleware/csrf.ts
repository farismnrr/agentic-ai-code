import { forbidden } from '#server/core/errors/http'
import { allowsMutationOrigin } from '../application/http-security'

export default defineEventHandler((event) => {
  const config = useRuntimeConfig()
  const allowed = allowsMutationOrigin({
    method: event.method,
    path: getRequestURL(event).pathname,
    origin: getHeader(event, 'origin'),
    siteUrl: config.public.siteUrl,
    authorization: getHeader(event, 'authorization'),
    secFetchSite: getHeader(event, 'sec-fetch-site')
  })
  if (!allowed) throw forbidden('Invalid request origin')
})
