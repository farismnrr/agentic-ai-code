export type ActivityStatus = 'started' | 'running' | 'ok' | 'error' | 'denied' | 'cancelled' | 'interrupted'
export type ActivityEvidence = 'exact' | 'summary' | 'unavailable' | 'not_applicable'

export interface ActivityItem {
  id: string
  occurredAt: string
  actor?: { label?: string, source?: string, channel?: string }
  clientInfo?: { name: string, version: string }
  operation: string
  category: string
  effects?: string[]
  target?: string
  status: ActivityStatus
  durationMs?: number
  affectedPaths?: string[]
  additions?: number
  deletions?: number
  evidence: ActivityEvidence
  result?: string
  complete?: boolean
  diffAvailable?: boolean
}

export interface ActivityResponse {
  items: ActivityItem[]
  nextCursor: string | null
  hasMore: boolean
  degraded?: boolean
}

export interface ActivityDetail extends ActivityItem {
  startedAt: string
  finishedAt?: string
  sourceSequence: number
}

export interface ActivityDiff {
  files?: Array<{ path: string, hunks: string[], additions?: number, deletions?: number }>
  complete?: boolean
}
