import type { BackgroundTaskMetadata, SubagentResult } from '../../../shared/types/subagents.ts'

const MAX_RUNS = 128
const MAX_CHILDREN = 24
const MAX_ISSUES = 64
const MAX_TEXT = 512
const MAX_REFS = 16

type Severity = 'P0' | 'P1' | 'P2' | 'info'
type WriterState = 'produced' | 'reviewed' | 'accepted' | 'integrated' | 'delivered'

export interface WriterIdentity { branch: string, base_commit: string, head_commit: string, dirty: boolean }
export interface ReconciliationChild { task: BackgroundTaskMetadata, writer?: WriterIdentity }
export interface ReconciliationIssue { key: string, severity: Severity, statement: string, evidence_refs: string[], source_task_ids: string[], conflict: boolean }
export interface ReconciliationWriter { task_id: string, branch: string, base_commit: string, head_commit: string, dirty: boolean, state: WriterState }
export interface ReconciliationSnapshot { generation: string, issues: ReconciliationIssue[], writers: ReconciliationWriter[], blockers: string[], delivery_ready: boolean, updated_at: number }

type Entry = ReconciliationSnapshot & { user_id: string, conversation_id: string }
const runs = new Map<string, Entry>()
const keyFor = (userId: string, conversationId: string) => `${userId}\0${conversationId}`
const bounded = (value: string) => value.replaceAll(/\p{Cc}/gu, ' ').trim().slice(0, MAX_TEXT)

export function reconcileChildren(input: { userId: string, conversationId: string, generation: string, children: ReconciliationChild[], now?: number }): ReconciliationSnapshot {
  const now = input.now ?? Date.now()
  if (!input.generation || input.children.length === 0 || input.children.length > MAX_CHILDREN) throw new Error('invalid reconciliation input')
  const issues = collectIssues(input.children)
  const writers = input.children.flatMap(({ task, writer }) => writer ? [{ task_id: task.task_id, branch: bounded(writer.branch), base_commit: writer.base_commit, head_commit: writer.head_commit, dirty: writer.dirty, state: 'produced' as const }] : [])
  const entry: Entry = { user_id: input.userId, conversation_id: input.conversationId, generation: input.generation, issues, writers, blockers: [], delivery_ready: false, updated_at: now }
  recompute(entry)
  runs.set(keyFor(input.userId, input.conversationId), entry)
  evict()
  return publicEntry(entry)
}

export function getReconciliation(userId: string, conversationId: string, generation: string): ReconciliationSnapshot | undefined {
  const entry = runs.get(keyFor(userId, conversationId))
  return entry?.generation === generation ? publicEntry(entry) : undefined
}

export function advanceWriter(input: { userId: string, conversationId: string, generation: string, taskId: string, expectedHead: string, action: 'review' | 'accept' | 'integrate', currentWriter?: WriterIdentity, now?: number }): ReconciliationSnapshot {
  const entry = requireEntry(input.userId, input.conversationId, input.generation)
  const writer = entry.writers.find(item => item.task_id === input.taskId)
  if (!writer || writer.head_commit !== input.expectedHead || writer.dirty || (input.currentWriter && !sameWriterIdentity(writer, input.currentWriter))) throw new Error('stale or dirty writer evidence')
  if (input.action === 'review' && writer.state !== 'produced') throw new Error('writer transition is invalid')
  if (input.action === 'accept' && writer.state !== 'reviewed') throw new Error('writer transition is invalid')
  if (input.action === 'integrate' && writer.state !== 'accepted') throw new Error('writer transition is invalid')
  writer.state = input.action === 'review' ? 'reviewed' : input.action === 'accept' ? 'accepted' : 'integrated'
  entry.updated_at = input.now ?? Date.now()
  recompute(entry)
  return publicEntry(entry)
}

function sameWriterIdentity(expected: ReconciliationWriter, actual: WriterIdentity) {
  return expected.branch === actual.branch && expected.base_commit === actual.base_commit && expected.head_commit === actual.head_commit && expected.dirty === actual.dirty
}

export function markDelivered(input: { userId: string, conversationId: string, generation: string, now?: number }): ReconciliationSnapshot {
  const entry = requireEntry(input.userId, input.conversationId, input.generation)
  recompute(entry)
  if (!entry.delivery_ready) throw new Error('reconciliation is not delivery ready')
  for (const writer of entry.writers) writer.state = 'delivered'
  entry.updated_at = input.now ?? Date.now()
  recompute(entry)
  return publicEntry(entry)
}

export function resetReconciliationForTests() {
  runs.clear()
}

function collectIssues(children: ReconciliationChild[]): ReconciliationIssue[] {
  const byKey = new Map<string, ReconciliationIssue>()
  const evidenceStatements = new Map<string, Set<string>>()
  for (const { task } of children) {
    const result = task.result as SubagentResult | undefined
    if (!result) continue
    const refs = [...new Set(result.evidence.map(item => bounded(item.reference)).filter(Boolean))].slice(0, MAX_REFS)
    for (const raw of result.findings.slice(0, MAX_ISSUES)) {
      const statement = bounded(raw)
      if (!statement) continue
      const severity = classifySeverity(statement)
      const key = `${severity}:${statement.toLowerCase()}`
      const existing = byKey.get(key)
      if (existing) {
        if (!existing.source_task_ids.includes(task.task_id)) existing.source_task_ids.push(task.task_id)
        existing.evidence_refs = [...new Set([...existing.evidence_refs, ...refs])].slice(0, MAX_REFS)
      } else if (byKey.size < MAX_ISSUES) {
        byKey.set(key, { key, severity, statement, evidence_refs: refs, source_task_ids: [task.task_id], conflict: false })
      }
      for (const ref of refs) {
        const values = evidenceStatements.get(ref) ?? new Set<string>()
        values.add(statement.toLowerCase())
        evidenceStatements.set(ref, values)
      }
    }
  }
  const issues = [...byKey.values()]
  for (const issue of issues) issue.conflict = issue.evidence_refs.some(ref => (evidenceStatements.get(ref)?.size ?? 0) > 1)
  return issues
}

function classifySeverity(statement: string): Severity {
  const match = statement.match(/^\s*(P[0-2])\b/i)?.[1]?.toUpperCase()
  return match === 'P0' || match === 'P1' || match === 'P2' ? match : 'info'
}

function recompute(entry: Entry) {
  const blockers: string[] = []
  if (entry.issues.some(issue => issue.severity === 'P0' || issue.severity === 'P1')) blockers.push('high_severity_finding')
  if (entry.issues.some(issue => issue.conflict)) blockers.push('reviewer_disagreement')
  if (entry.writers.some(writer => writer.dirty)) blockers.push('dirty_writer')
  if (entry.writers.some(writer => writer.state !== 'integrated' && writer.state !== 'delivered')) blockers.push('unintegrated_writer')
  entry.blockers = [...new Set(blockers)]
  entry.delivery_ready = entry.blockers.length === 0
}

function requireEntry(userId: string, conversationId: string, generation: string) {
  const entry = runs.get(keyFor(userId, conversationId))
  if (!entry || entry.generation !== generation) throw new Error('stale reconciliation generation')
  return entry
}

function publicEntry(entry: Entry): ReconciliationSnapshot {
  const { user_id: _, conversation_id: __, ...safe } = entry
  return structuredClone(safe)
}

function evict() {
  if (runs.size <= MAX_RUNS) return
  const oldest = [...runs.entries()].sort((a, b) => a[1].updated_at - b[1].updated_at).slice(0, runs.size - MAX_RUNS)
  for (const [key] of oldest) runs.delete(key)
}
