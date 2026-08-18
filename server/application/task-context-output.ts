/* eslint-disable @stylistic/max-statements-per-line */
import { createHmac, randomUUID, timingSafeEqual } from 'node:crypto'
import { tool } from 'ai'
import { z } from 'zod'

export const TASK_CAPS = { count: 32, title: 160, note: 512, dependencies: 16, ttlMs: 30 * 60 * 1000 } as const
export const RESULT_REF_CAPS = { entries: 64, bytes: 512 * 1024, itemBytes: 32 * 1024, ttlMs: 15 * 60 * 1000 } as const
export const CONTINUATION_CAPS = { page: 100, total: 1_000, ttlMs: 5 * 60 * 1000 } as const

export type TaskStatus = 'pending' | 'in_progress' | 'blocked' | 'completed' | 'cancelled'
export interface TaskItem { id: string, title: string, status: TaskStatus, depends_on: string[], short_note?: string, updated_at: number }
export interface TaskLedger { userId: string, conversationId: string, sessionId: string, tasks: TaskItem[], updatedAt: number }

const taskStores = new Map<string, TaskLedger>()
const keyFor = (userId: string, conversationId: string, sessionId: string) => `${userId}\0${conversationId}\0${sessionId}`
const bounded = (value: unknown, max: number) => typeof value === 'string' ? value.trim().slice(0, max) : ''
const statusValues = new Set<TaskStatus>(['pending', 'in_progress', 'blocked', 'completed', 'cancelled'])

export function taskLedgerFor(userId: string, conversationId: string, sessionId: string, now = Date.now()): TaskLedger {
  const key = keyFor(userId, conversationId, sessionId)
  const existing = taskStores.get(key)
  if (existing && now - existing.updatedAt <= TASK_CAPS.ttlMs) return structuredClone(existing)
  const fresh = { userId, conversationId, sessionId, tasks: [], updatedAt: now }
  taskStores.set(key, fresh)
  return structuredClone(fresh)
}

export function updateTaskLedger(input: { userId: string, conversationId: string, sessionId: string, tasks: unknown, now?: number }): TaskLedger {
  const now = input.now ?? Date.now()
  if (!Array.isArray(input.tasks) || input.tasks.length > TASK_CAPS.count) throw new Error('task update exceeds bounded task count')
  const ids = new Set<string>()
  const tasks: TaskItem[] = input.tasks.map((raw, index) => {
    if (!raw || typeof raw !== 'object') throw new Error('malformed task update')
    const value = raw as Record<string, unknown>
    const id = bounded(value.id, 64)
    const title = bounded(value.title, TASK_CAPS.title)
    const status = value.status
    if (!id || !title || ids.has(id) || typeof status !== 'string' || !statusValues.has(status as TaskStatus)) throw new Error('malformed task update')
    ids.add(id)
    const dependencies = Array.isArray(value.depends_on) ? value.depends_on.map(item => bounded(item, 64)).filter(Boolean).slice(0, TASK_CAPS.dependencies) : []
    if (dependencies.some(dep => dep === id)) throw new Error('task dependency cycle')
    return { id, title, status: status as TaskStatus, depends_on: dependencies, short_note: bounded(value.short_note, TASK_CAPS.note) || undefined, updated_at: now + index / 1000 }
  })
  if (tasks.some(task => task.depends_on.some(dep => !ids.has(dep)))) throw new Error('unknown task dependency')
  const visiting = new Set<string>(), visited = new Set<string>()
  const visit = (id: string): void => {
    if (visiting.has(id)) throw new Error('task dependency cycle')
    if (visited.has(id)) return
    visiting.add(id)
    for (const dep of tasks.find(task => task.id === id)?.depends_on ?? []) visit(dep)
    visiting.delete(id); visited.add(id)
  }
  tasks.forEach(task => visit(task.id))
  if (tasks.filter(task => task.status === 'in_progress').length > 1) throw new Error('only one task may be in progress')
  const ledger = { userId: input.userId, conversationId: input.conversationId, sessionId: input.sessionId, tasks, updatedAt: now }
  taskStores.set(keyFor(input.userId, input.conversationId, input.sessionId), ledger)
  return structuredClone(ledger)
}

const continuationSecret = process.env.AI_CODE_CONTINUATION_SECRET ?? 'development-only-continuation-secret'
const sign = (body: string) => createHmac('sha256', continuationSecret).update(body).digest('base64url')
export interface ContinuationClaims { tool: string, query: string, scope: string, limit: number, offset: number, retrieved: number, expiresAt: number, owner?: string, snapshot?: string }
export function issueContinuation(claims: Omit<ContinuationClaims, 'expiresAt'> & { expiresAt?: number }): string {
  if (!Number.isInteger(claims.limit) || claims.limit < 1 || claims.limit > CONTINUATION_CAPS.page || claims.offset < 0 || claims.retrieved < 0 || claims.retrieved > CONTINUATION_CAPS.total) throw new Error('invalid continuation claims')
  const body = Buffer.from(JSON.stringify({ ...claims, expiresAt: claims.expiresAt ?? Date.now() + CONTINUATION_CAPS.ttlMs })).toString('base64url')
  return `${body}.${sign(body)}`
}
export function consumeContinuation(token: unknown, expected: Omit<ContinuationClaims, 'offset' | 'retrieved' | 'expiresAt' | 'snapshot'> & { owner?: string, snapshot?: string }, now = Date.now()): ContinuationClaims {
  if (typeof token !== 'string' || token.length > 4096) throw new Error('invalid continuation')
  const [body, mac] = token.split('.')
  if (!body || !mac) throw new Error('invalid continuation')
  const expectedMac = sign(body)
  if (mac.length !== expectedMac.length || !timingSafeEqual(Buffer.from(mac), Buffer.from(expectedMac))) throw new Error('invalid continuation')
  let claims: ContinuationClaims
  try { claims = JSON.parse(Buffer.from(body, 'base64url').toString('utf8')) as ContinuationClaims } catch { throw new Error('invalid continuation') }
  if (claims.tool !== expected.tool || claims.query !== expected.query || claims.scope !== expected.scope || claims.limit !== expected.limit || claims.owner !== expected.owner || claims.snapshot !== expected.snapshot || claims.expiresAt <= now || claims.offset < 0 || claims.retrieved + claims.limit > CONTINUATION_CAPS.total) throw new Error('stale continuation')
  return claims
}

type RefEntry = { owner: string, value: string, bytes: number, expiresAt: number, sequence: number }
const refs = new Map<string, RefEntry>(); let sequence = 0; let refBytes = 0
export function putResultRef(owner: string, value: string, now = Date.now()): string {
  if (Buffer.byteLength(value) > RESULT_REF_CAPS.itemBytes) throw new Error('result reference item exceeds maximum')
  evictResultRefs(now)
  while (refs.size >= RESULT_REF_CAPS.entries || refBytes + Buffer.byteLength(value) > RESULT_REF_CAPS.bytes) evictResultRefs(now, true)
  const id = `rr_${randomUUID().replaceAll('-', '')}`
  const bytes = Buffer.byteLength(value); refs.set(id, { owner, value, bytes, expiresAt: now + RESULT_REF_CAPS.ttlMs, sequence: sequence++ }); refBytes += bytes
  return id
}
export function getResultRef(owner: string, id: string, now = Date.now()): string | undefined { evictResultRefs(now); const entry = refs.get(id); return entry?.owner === owner ? entry.value : undefined }
export function evictResultRefs(now = Date.now(), forceOldest = false) { for (const [id, entry] of refs) if (entry.expiresAt <= now) { refs.delete(id); refBytes -= entry.bytes }; if (forceOldest && refs.size) { const oldest = [...refs.entries()].sort((a, b) => a[1].sequence - b[1].sequence)[0]; if (oldest) { refs.delete(oldest[0]); refBytes -= oldest[1].bytes } } }

export type OutputClass = 'inline_small' | 'paginated_medium' | 'summarized_large' | 'retained_failure'
export const classifyOutput = (bytes: number, failed = false): OutputClass => failed ? 'retained_failure' : bytes <= 16 * 1024 ? 'inline_small' : bytes <= 128 * 1024 ? 'paginated_medium' : 'summarized_large'
export function inspectContext(input: { contextWindow?: number | null, usedTokens?: number | null, maxOutputTokens?: number | null, summary?: string | null, summaryAgeMs?: number | null, childCount?: number, backgroundCount?: number }) {
  const exact = input.usedTokens != null
  const reserved = input.maxOutputTokens ?? null
  const headroom = input.contextWindow != null && input.usedTokens != null ? Math.max(0, input.contextWindow - input.usedTokens - (reserved ?? 0)) : null
  return { contextWindow: input.contextWindow ?? null, usedTokens: input.usedTokens ?? null, usedTokensKind: exact ? 'exact_provider_accounted' : 'estimated', reservedOutputTokens: reserved, headroom, summaryPresent: Boolean(input.summary), summaryAgeMs: input.summary ? input.summaryAgeMs ?? null : null, activeChildren: Math.min(32, Math.max(0, input.childCount ?? 0)), activeBackgroundTasks: Math.min(32, Math.max(0, input.backgroundCount ?? 0)), pressure: headroom != null && input.contextWindow ? headroom / input.contextWindow < 0.15 : 'unknown' as const }
}

export function resetTaskContextStoresForTests() { taskStores.clear(); refs.clear(); refBytes = 0; sequence = 0 }

export function buildTaskUpdateTool(input: { userId: string, conversationId: string }) {
  return tool({
    description: 'Maintain a compact ephemeral progress ledger. This is UI state only and never proves validation, hooks, Git delivery, or repository completion.',
    inputSchema: z.object({ tasks: z.array(z.object({ id: z.string().min(1).max(64), title: z.string().min(1).max(TASK_CAPS.title), status: z.enum(['pending', 'in_progress', 'blocked', 'completed', 'cancelled']), depends_on: z.array(z.string().max(64)).max(TASK_CAPS.dependencies).default([]), short_note: z.string().max(TASK_CAPS.note).optional() })).max(TASK_CAPS.count) }),
    execute: async ({ tasks }) => updateTaskLedger({ ...input, sessionId: input.conversationId, tasks })
  })
}
