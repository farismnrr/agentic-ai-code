import { badRequest } from '#server/core/errors/http'

const SAFE_METHODS = new Set(['GET', 'HEAD', 'OPTIONS'])

export default defineEventHandler((event) => {
  if (SAFE_METHODS.has(event.method.toUpperCase()) || !getRequestURL(event).pathname.startsWith('/api/')) return
  const contentType = getHeader(event, 'content-type')
  if (!contentType) return
  if (!contentType.toLowerCase().startsWith('application/json')) {
    throw badRequest('JSON content is required')
  }
})
