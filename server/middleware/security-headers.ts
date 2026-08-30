import { securityHeadersForPath } from '../application/http-security'

export default defineEventHandler((event) => {
  const headers = securityHeadersForPath(getRequestURL(event).pathname)
  for (const [name, value] of Object.entries(headers)) setResponseHeader(event, name, value)
})
