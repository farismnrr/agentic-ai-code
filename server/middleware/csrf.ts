import { forbidden } from '#server/core/errors/http'

const SAFE_METHODS = new Set(['GET', 'HEAD', 'OPTIONS'])

export default defineEventHandler((event) => {
  const method = event.method.toUpperCase()
  if (SAFE_METHODS.has(method) || !getRequestURL(event).pathname.startsWith('/api/')) return

  const origin = getHeader(event, 'origin')
  if (!origin) return
  const config = useRuntimeConfig()
  const origins = [config.public.siteUrl].filter((value): value is string => Boolean(value))
  const allowed = new Set(origins.map(value => new URL(value).origin))
  if (!allowed.has(origin)) throw forbidden('Invalid request origin')
})
