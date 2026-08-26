import { badRequest } from '#server/core/errors/http'

function decodeCursor(value: unknown) {
  if (typeof value !== 'string' || value.length > 512) return undefined
  try {
    const parsed = JSON.parse(Buffer.from(value, 'base64url').toString('utf8')) as { startedAt?: string, id?: string }
    if (!parsed.startedAt || typeof parsed.id !== 'string' || parsed.id.length === 0 || parsed.id.length > 64) return undefined
    const startedAt = new Date(parsed.startedAt)
    return Number.isNaN(startedAt.getTime()) ? undefined : { startedAt, id: parsed.id }
  } catch {
    return undefined
  }
}

function encodeCursor(cursor: { startedAt: Date, id: string }) {
  return Buffer.from(JSON.stringify({ startedAt: cursor.startedAt.toISOString(), id: cursor.id })).toString('base64url')
}

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const workspaceId = getRouterParam(event, 'id')
  if (!workspaceId) throw badRequest('Missing workspace id')
  const query = getQuery(event)
  const limit = typeof query.limit === 'string' ? Number(query.limit) : undefined
  if (limit !== undefined && (!Number.isSafeInteger(limit) || limit < 1 || limit > 100)) throw badRequest('Invalid activity limit')
  const categories = ['filesystem', 'search', 'terminal', 'git', 'code', 'delegated', 'network', 'workspace', 'other']
  const statuses = ['started', 'running', 'ok', 'error', 'denied', 'cancelled', 'interrupted']
  if (typeof query.category === 'string' && query.category !== 'all' && !categories.includes(query.category)) throw badRequest('Invalid activity category')
  if (typeof query.status === 'string' && query.status !== 'all' && !statuses.includes(query.status)) throw badRequest('Invalid activity status')
  if (query.cursor !== undefined && decodeCursor(query.cursor) === undefined) throw badRequest('Invalid activity cursor')
  const since = typeof query.since === 'string' ? new Date(query.since) : undefined
  if (since && Number.isNaN(since.getTime())) throw badRequest('Invalid activity since value')
  const result = await event.context.application.activity.list(session.user.id, workspaceId, {
    limit,
    cursor: decodeCursor(query.cursor),
    since,
    query: typeof query.q === 'string' ? query.q.slice(0, 120) : undefined,
    category: typeof query.category === 'string' && query.category !== 'all' ? query.category.slice(0, 64) : undefined,
    status: typeof query.status === 'string' && query.status !== 'all' ? query.status as never : undefined
  })
  setResponseHeader(event, 'Cache-Control', 'private, no-store')
  return { items: result.items, nextCursor: result.nextCursor ? encodeCursor(result.nextCursor) : null, hasMore: Boolean(result.nextCursor), degraded: false }
})
