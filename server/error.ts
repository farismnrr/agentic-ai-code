export default defineNitroErrorHandler((error, event) => {
  const data = (error.data && typeof error.data === 'object' ? error.data : {}) as Record<string, unknown>
  const isProblem = data.problem === true
  const status = isProblem ? error.statusCode : 500

  const extensionFields = (extData: Record<string, unknown>) => {
    const { problem, type, title, status, detail, ...rest } = extData
    return rest
  }

  const body = isProblem
    ? { type: data.type, title: data.title, status, detail: data.detail, instance: event.path, ...extensionFields(data) }
    : { type: 'about:blank', title: 'Internal Server Error', status: 500, instance: event.path }

  // Full detail (message asli, stack, error object) HANYA ke server log — tidak pernah ke client.
  if (!isProblem) console.error('[unhandled]', error)

  setResponseHeader(event, 'Content-Type', 'application/problem+json')
  if (isProblem && data.retryAfter) {
    setResponseHeader(event, 'Retry-After', Number(data.retryAfter))
  }
  setResponseStatus(event, status)
  return send(event, JSON.stringify(body))
})
