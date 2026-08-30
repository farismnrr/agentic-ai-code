import { badRequest } from '#server/core/errors/http'
import { allowsMutationContentType } from '../application/http-security'

export default defineEventHandler((event) => {
  const allowed = allowsMutationContentType({
    method: event.method,
    path: getRequestURL(event).pathname,
    contentType: getHeader(event, 'content-type'),
    contentLength: getHeader(event, 'content-length'),
    transferEncoding: getHeader(event, 'transfer-encoding')
  })
  if (!allowed) throw badRequest('JSON content is required')
})
