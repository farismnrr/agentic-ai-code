export type ActivityTerminalStatus = 'ok' | 'error' | 'denied' | 'cancelled' | 'interrupted'
export type ActivityStatus = 'started' | 'running' | ActivityTerminalStatus
export type ActivityEvidence = 'exact' | 'summary' | 'unavailable' | 'not_applicable'

export interface ActivityPresentation {
  target?: string | null
  action?: string | null
  summary?: string | null
  resultDetail?: string | null
  resultClass?: string | null
  evidence: ActivityEvidence
  payloadReference?: string | null
  complete: boolean
}

export interface ActivityIngressEvent {
  recordId: string
  sourceId: string
  contractVersion: string
  activityId: string
  sourceSequence: number
  status: ActivityStatus
  toolId: string
  category: string
  effects: string[]
  workspaceRootFingerprint?: string | null
  actor: { label: string, source?: string | null, channel?: string | null }
  clientInfo?: { name: string, version: string } | null
  occurredAtMs: number
  durationMs?: number | null
  presentation: ActivityPresentation
  payload?: ActivityPayload
}

export interface ActivityPayload {
  kind: string
  version: string
  value: string
  byteCount: number
}

export interface ActivityItem {
  id: string
  occurredAt: string
  actor: { label: string, source?: string, channel?: string }
  clientInfo?: { name: string, version: string }
  operation: string
  category: string
  effects: string[]
  target?: string
  action?: string
  status: ActivityStatus
  durationMs?: number
  affectedPaths?: string[]
  additions?: number
  deletions?: number
  evidence: ActivityEvidence
  result?: string
  resultDetail?: string
  complete: boolean
  diffAvailable: boolean
}

export interface ActivityDetail extends ActivityItem {
  startedAt: string
  finishedAt?: string
  sourceSequence: number
}

export interface ActivityCursor { startedAt: Date, id: string }
export interface ActivityListOptions {
  limit: number
  cursor?: ActivityCursor
  since?: Date
  query?: string
  category?: string
  status?: ActivityStatus
}

export interface ActivityPort {
  enroll(userId: string, input: { label: string, kind: string, deviceId?: string }): Promise<{ id: string, token: string, tokenPrefix: string }>
  listSources(userId: string): Promise<Array<{ id: string, label: string, kind: string, deviceId?: string, tokenPrefix: string, createdAt: string, lastSeenAt?: string, revokedAt?: string }>>
  revoke(userId: string, sourceId: string): Promise<void>
  bind(userId: string, sourceId: string, workspaceId: string): Promise<void>
  ingest(token: string, events: ActivityIngressEvent[]): Promise<{ accepted: string[], duplicates: string[] }>
  list(userId: string, workspaceId: string, options: ActivityListOptions): Promise<{ items: ActivityItem[], nextCursor?: ActivityCursor }>
  detail(userId: string, workspaceId: string, activityId: string): Promise<ActivityDetail>
  diff(userId: string, workspaceId: string, activityId: string): Promise<{ files: Array<{ path: string, hunks: string[], additions: number, deletions: number }>, complete: boolean }>
  clear(userId: string, workspaceId: string): Promise<void>
  retain(before: Date, limit: number): Promise<number>
}

export function createActivityUseCases(port: ActivityPort) {
  return {
    enroll: port.enroll,
    listSources: port.listSources,
    revoke: port.revoke,
    bind: port.bind,
    ingest: port.ingest,
    list: (userId: string, workspaceId: string, options: Omit<ActivityListOptions, 'limit'> & { limit?: number }) => port.list(userId, workspaceId, { ...options, limit: Math.min(Math.max(options.limit ?? 50, 1), 100) }),
    detail: port.detail,
    diff: port.diff,
    clear: port.clear,
    retain: port.retain
  }
}
