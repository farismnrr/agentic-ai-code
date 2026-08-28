import { createHash, randomBytes, randomUUID } from 'node:crypto'
import { and, asc, desc, eq, gt, ilike, inArray, isNull, lt, or } from 'drizzle-orm'
import { useDb } from './connection'
import { relayActivitySources, relayActivityWorkspaceBindings, workspaceActivity, workspaceActivityPayloads, workspaces } from '../../database/schema'
import type { ActivityEvidence, ActivityIngressEvent, ActivityItem, ActivityPort, ActivityStatus } from '../../application/activity'
import { decryptActivityPayload, encryptActivityPayload } from '../activity/crypto'
import { resolveWorkspacePath } from '../filesystem/browse'
import fs from 'node:fs/promises'

const MAX_PAYLOAD_BYTES = 512 * 1024
const hash = (token: string) => createHash('sha256').update(token).digest('hex')

type ActivityRow = typeof workspaceActivity.$inferSelect

export const activityDatabase: ActivityPort = {
  async enroll(userId, input) {
    const token = randomBytes(32).toString('base64url')
    const [source] = await useDb().insert(relayActivitySources).values({
      id: randomUUID(),
      userId,
      deviceId: input.deviceId,
      label: input.label.slice(0, 80),
      kind: input.kind.slice(0, 32),
      tokenHash: hash(token),
      tokenPrefix: token.slice(0, 12)
    }).returning({ id: relayActivitySources.id, tokenPrefix: relayActivitySources.tokenPrefix })
    if (!source) throw new Error('Activity source enrollment failed')
    return { ...source, token }
  },

  async listSources(userId) {
    const rows = await useDb().select({
      id: relayActivitySources.id,
      label: relayActivitySources.label,
      kind: relayActivitySources.kind,
      deviceId: relayActivitySources.deviceId,
      tokenPrefix: relayActivitySources.tokenPrefix,
      createdAt: relayActivitySources.createdAt,
      lastSeenAt: relayActivitySources.lastSeenAt,
      revokedAt: relayActivitySources.revokedAt
    }).from(relayActivitySources).where(eq(relayActivitySources.userId, userId)).orderBy(desc(relayActivitySources.createdAt))
    return rows.map(row => ({
      ...row,
      deviceId: row.deviceId ?? undefined,
      createdAt: row.createdAt.toISOString(),
      lastSeenAt: row.lastSeenAt?.toISOString(),
      revokedAt: row.revokedAt?.toISOString()
    }))
  },

  async revoke(userId, sourceId) {
    await useDb().update(relayActivitySources).set({ revokedAt: new Date() }).where(and(eq(relayActivitySources.id, sourceId), eq(relayActivitySources.userId, userId)))
  },

  async bind(userId, sourceId, workspaceId) {
    const db = useDb()
    const [owned] = await db.select({ id: workspaces.id, path: workspaces.path }).from(workspaces).where(and(eq(workspaces.id, workspaceId), eq(workspaces.userId, userId))).limit(1)
    const [source] = await db.select({ id: relayActivitySources.id }).from(relayActivitySources).where(and(eq(relayActivitySources.id, sourceId), eq(relayActivitySources.userId, userId), isNull(relayActivitySources.revokedAt))).limit(1)
    if (!owned || !source) throw new Error('Activity workspace binding failed')
    const canonicalRoot = await fs.realpath(await resolveWorkspacePath(owned.path))
    const rootFingerprint = createHash('sha256').update(canonicalRoot).digest('hex')
    await db.insert(relayActivityWorkspaceBindings).values({ sourceId, workspaceId, rootFingerprint }).onConflictDoUpdate({ target: [relayActivityWorkspaceBindings.sourceId, relayActivityWorkspaceBindings.rootFingerprint], set: { workspaceId, lastSeenAt: new Date() } })
  },

  async ingest(token, events) {
    if (events.length > 100 || events.length === 0) throw new Error('Activity batch limit exceeded')
    const db = useDb()
    const [source] = await db.select().from(relayActivitySources).where(and(eq(relayActivitySources.tokenHash, hash(token)), isNull(relayActivitySources.revokedAt))).limit(1)
    if (!source) throw new Error('Activity ingestion rejected')
    const accepted: string[] = []
    const duplicates: string[] = []
    await db.transaction(async (tx) => {
      for (const event of events) {
        validateIngress(event)
        if (event.sourceId !== (source.sourceKey ?? event.sourceId)) throw new Error('Activity ingestion rejected')
        if (!source.sourceKey) {
          await tx.update(relayActivitySources).set({ sourceKey: event.sourceId }).where(eq(relayActivitySources.id, source.id))
          source.sourceKey = event.sourceId
        }
        const rootFingerprint = event.workspaceRootFingerprint
        if (!rootFingerprint) {
          accepted.push(event.recordId)
          continue
        }
        const [binding] = await tx.select({ workspaceId: relayActivityWorkspaceBindings.workspaceId, clearThroughSequence: relayActivityWorkspaceBindings.clearThroughSequence }).from(relayActivityWorkspaceBindings).where(and(eq(relayActivityWorkspaceBindings.sourceId, source.id), eq(relayActivityWorkspaceBindings.rootFingerprint, rootFingerprint))).limit(1)
        // A relay can legitimately execute inside temporary worktrees or
        // other authorized roots that are not represented by an AI Code
        // workspace. The source credential is already authenticated above;
        // an unknown root fingerprint therefore means "not part of this
        // workspace read model", not "revoke/poison the whole source".
        // Acknowledge and discard that record so it cannot be mapped to an
        // owned workspace, while allowing later bound events in the same
        // batch to continue exporting.
        if (!binding) {
          accepted.push(event.recordId)
          continue
        }
        if (event.sourceSequence <= binding.clearThroughSequence) {
          accepted.push(event.recordId)
          continue
        }
        const [current] = await tx.select().from(workspaceActivity).where(and(eq(workspaceActivity.sourceId, source.id), eq(workspaceActivity.activityId, event.activityId))).limit(1)
        if (current && event.sourceSequence <= current.sourceSequence) {
          duplicates.push(event.recordId)
          continue
        }
        if (current && !transitionAllowed(current.status, event.status)) throw new Error('Activity lifecycle transition rejected')
        const payloadSecret = event.payload ? activitySecretOrUndefined() : undefined
        const evidence = evidenceMetadata(event)
        if (event.presentation.evidence === 'exact' && (!event.payload || !payloadSecret)) {
          evidence.evidence = 'unavailable'
          evidence.complete = false
          evidence.affectedPaths = []
          evidence.additions = 0
          evidence.deletions = 0
        }
        const values = {
          sourceId: source.id,
          activityId: event.activityId,
          workspaceId: binding.workspaceId,
          sourceSequence: event.sourceSequence,
          contractVersion: event.contractVersion,
          actor: event.actor.label,
          actorSource: event.actor.source?.slice(0, 80),
          channel: event.actor.channel ?? 'relay',
          clientInfoName: event.clientInfo?.name?.slice(0, 256),
          clientInfoVersion: event.clientInfo?.version?.slice(0, 64),
          tool: event.toolId,
          category: event.category,
          effects: event.effects.slice(0, 16),
          status: event.status,
          target: event.presentation.target ?? 'Workspace operation',
          startedAt: current?.startedAt ?? new Date(event.occurredAtMs),
          finishedAt: isTerminal(event.status) ? new Date(event.occurredAtMs) : current?.finishedAt,
          durationMs: event.durationMs ?? current?.durationMs,
          changeEvidence: evidence,
          occurredAt: new Date(event.occurredAtMs)
        }
        let activityRow: { id: string } | undefined
        if (current) {
          ;[activityRow] = await tx.update(workspaceActivity).set(values).where(eq(workspaceActivity.id, current.id)).returning({ id: workspaceActivity.id })
        } else {
          ;[activityRow] = await tx.insert(workspaceActivity).values(values).returning({ id: workspaceActivity.id })
        }
        if (!activityRow) throw new Error('Activity ingestion failed')
        if (event.payload && payloadSecret) await savePayload(tx, activityRow.id, source.id, event.activityId, event.payload, payloadSecret)
        accepted.push(event.recordId)
      }
      await tx.update(relayActivitySources).set({ lastSeenAt: new Date() }).where(eq(relayActivitySources.id, source.id))
    })
    return { accepted, duplicates }
  },

  async list(userId, workspaceId, options) {
    await assertWorkspaceOwner(userId, workspaceId)
    const predicates = [eq(workspaceActivity.workspaceId, workspaceId)]
    if (options.cursor) predicates.push(or(lt(workspaceActivity.startedAt, options.cursor.startedAt), and(eq(workspaceActivity.startedAt, options.cursor.startedAt), lt(workspaceActivity.id, options.cursor.id)))!)
    if (options.since) predicates.push(gt(workspaceActivity.occurredAt, options.since))
    if (options.query) predicates.push(or(ilike(workspaceActivity.tool, `%${escapeLike(options.query)}%`), ilike(workspaceActivity.target, `%${escapeLike(options.query)}%`), ilike(workspaceActivity.actor, `%${escapeLike(options.query)}%`), ilike(workspaceActivity.actorSource, `%${escapeLike(options.query)}%`))!)
    if (options.category) predicates.push(eq(workspaceActivity.category, options.category))
    if (options.status) predicates.push(eq(workspaceActivity.status, options.status))
    const rows = await useDb().select().from(workspaceActivity).where(and(...predicates)).orderBy(desc(workspaceActivity.startedAt), desc(workspaceActivity.id)).limit(options.limit + 1)
    const page = rows.slice(0, options.limit)
    const last = rows[options.limit]
    return { items: page.map(mapItem), nextCursor: last ? { startedAt: last.startedAt, id: last.id } : undefined }
  },

  async detail(userId, workspaceId, activityId) {
    await assertWorkspaceOwner(userId, workspaceId)
    const [row] = await useDb().select().from(workspaceActivity).where(and(eq(workspaceActivity.id, activityId), eq(workspaceActivity.workspaceId, workspaceId))).limit(1)
    if (!row) throw new Error('Workspace activity not found')
    return { ...mapItem(row), startedAt: row.startedAt.toISOString(), finishedAt: row.finishedAt?.toISOString(), sourceSequence: row.sourceSequence }
  },

  async diff(userId, workspaceId, activityId) {
    await assertWorkspaceOwner(userId, workspaceId)
    const [row] = await useDb().select().from(workspaceActivity).where(and(eq(workspaceActivity.id, activityId), eq(workspaceActivity.workspaceId, workspaceId))).limit(1)
    if (!row) throw new Error('Workspace activity not found')
    const payload = await useDb().select().from(workspaceActivityPayloads).where(and(eq(workspaceActivityPayloads.activityId, row.id), eq(workspaceActivityPayloads.payloadKind, 'activity_evidence'))).orderBy(desc(workspaceActivityPayloads.createdAt)).limit(1)
    if (!payload[0]) return { files: [], complete: false }
    try {
      const secret = activitySecretOrUndefined()
      if (!secret) throw new Error('Activity diff unavailable')
      const plaintext = decryptActivityPayload(payload[0].encryptedEnvelope, secret, `${row.sourceId}:${row.activityId}:activity_evidence`)
      const raw = Buffer.from(plaintext, 'utf8')
      if (raw.length !== payload[0].byteCount || createHash('sha256').update(raw).digest('hex') !== payload[0].checksum) throw new Error('Activity payload integrity check failed')
      const evidence = JSON.parse(plaintext) as { complete?: boolean, files?: Array<{ path: string, before?: string, after?: string }> }
      const files = (evidence.files ?? []).flatMap((file) => {
        const path = safeEvidencePath(file.path)
        return path ? [diffFile(path, file.before ?? '', file.after ?? '')] : []
      })
      return { files, complete: evidence.complete === true && files.length === (evidence.files ?? []).length }
    } catch {
      throw new Error('Activity diff unavailable')
    }
  },

  async clear(userId, workspaceId) {
    await assertWorkspaceOwner(userId, workspaceId)
    const db = useDb()
    await db.transaction(async (tx) => {
      const bindings = await tx.select({ sourceId: relayActivityWorkspaceBindings.sourceId }).from(relayActivityWorkspaceBindings).where(eq(relayActivityWorkspaceBindings.workspaceId, workspaceId))
      for (const binding of bindings) {
        const [latest] = await tx.select({ sequence: workspaceActivity.sourceSequence }).from(workspaceActivity).where(and(eq(workspaceActivity.workspaceId, workspaceId), eq(workspaceActivity.sourceId, binding.sourceId))).orderBy(desc(workspaceActivity.sourceSequence)).limit(1)
        await tx.update(relayActivityWorkspaceBindings).set({ clearThroughSequence: latest?.sequence ?? 0 }).where(and(eq(relayActivityWorkspaceBindings.workspaceId, workspaceId), eq(relayActivityWorkspaceBindings.sourceId, binding.sourceId)))
      }
      await tx.delete(workspaceActivity).where(eq(workspaceActivity.workspaceId, workspaceId))
    }, { isolationLevel: 'serializable' })
  },

  async retain(before, limit) {
    const rows = await useDb().select({ id: workspaceActivity.id }).from(workspaceActivity).where(lt(workspaceActivity.startedAt, before)).orderBy(asc(workspaceActivity.startedAt)).limit(Math.min(limit, 500))
    if (rows.length) await useDb().delete(workspaceActivity).where(inArray(workspaceActivity.id, rows.map(row => row.id)))
    return rows.length
  }
}

async function assertWorkspaceOwner(userId: string, workspaceId: string) {
  const [owned] = await useDb().select({ id: workspaces.id }).from(workspaces).where(and(eq(workspaces.id, workspaceId), eq(workspaces.userId, userId))).limit(1)
  if (!owned) throw new Error('Workspace activity not found')
}

type ActivityTransaction = Parameters<Parameters<ReturnType<typeof useDb>['transaction']>[0]>[0]

async function savePayload(tx: ActivityTransaction, rowId: string, sourceId: string, activityId: string, payload: NonNullable<ActivityIngressEvent['payload']>, secret: string) {
  const raw = Buffer.from(payload.value, 'base64')
  if (raw.toString('base64') !== payload.value) throw new Error('Activity payload encoding is invalid')
  if (raw.length > MAX_PAYLOAD_BYTES || raw.length !== payload.byteCount) throw new Error('Activity payload exceeds allowed bounds')
  const checksum = createHash('sha256').update(raw).digest('hex')
  const envelope = encryptActivityPayload(raw.toString('utf8'), secret, `${sourceId}:${activityId}:${payload.kind}`)
  await tx.insert(workspaceActivityPayloads).values({ activityId: rowId, payloadKind: payload.kind.slice(0, 40), payloadVersion: payload.version.slice(0, 24), encryptedEnvelope: envelope, checksum, byteCount: raw.length }).onConflictDoUpdate({ target: [workspaceActivityPayloads.activityId, workspaceActivityPayloads.payloadKind, workspaceActivityPayloads.chunkIndex], set: { encryptedEnvelope: envelope, checksum, byteCount: raw.length, createdAt: new Date() } })
}

function validateIngress(event: ActivityIngressEvent) {
  const categories = ['filesystem', 'search', 'terminal', 'git', 'code', 'delegated', 'network', 'workspace', 'other']
  const effects = ['workspace_read', 'workspace_write', 'workspace_delete', 'process_exec', 'network_read', 'network_write', 'git_read', 'external_mutation', 'privileged_bridge']
  const statuses = ['started', 'running', 'ok', 'error', 'denied', 'cancelled', 'interrupted']
  if (event.contractVersion !== 'activity.event.v1' || !bounded(event.recordId, 320) || !bounded(event.activityId, 256) || !bounded(event.sourceId, 256) || !Number.isSafeInteger(event.sourceSequence) || event.sourceSequence < 1 || event.sourceSequence > 2147483647 || !statuses.includes(event.status) || !bounded(event.toolId, 256) || !categories.includes(event.category) || event.effects.length > 64 || event.effects.some(effect => !effects.includes(effect)) || !bounded(event.actor.label, 256) || !boundedOptional(event.actor.source, 256) || !boundedOptional(event.actor.channel, 256) || !boundedOptional(event.clientInfo?.name, 256) || !boundedOptional(event.clientInfo?.version, 64) || Boolean(event.clientInfo) !== Boolean(event.clientInfo?.name && event.clientInfo?.version) || !Number.isSafeInteger(event.occurredAtMs) || !boundedOptional(event.presentation.target, 4096) || !boundedOptional(event.presentation.action, 256) || !boundedOptional(event.presentation.summary, 256) || !boundedMultilineOptional(event.presentation.resultDetail, 8192) || !boundedOptional(event.presentation.resultClass, 256) || !boundedOptional(event.presentation.payloadReference, 256) || (event.durationMs !== undefined && event.durationMs !== null && (!Number.isSafeInteger(event.durationMs) || event.durationMs < 0 || event.durationMs > 2147483647)) || (event.workspaceRootFingerprint !== undefined && event.workspaceRootFingerprint !== null && !/^[a-f0-9]{64}$/.test(event.workspaceRootFingerprint))) throw new Error('Activity ingestion rejected')
  if (!['exact', 'summary', 'unavailable', 'not_applicable'].includes(event.presentation.evidence) || typeof event.presentation.complete !== 'boolean') throw new Error('Activity ingestion rejected')
  if (event.payload && (!bounded(event.payload.kind, 40) || !bounded(event.payload.version, 24) || !Number.isSafeInteger(event.payload.byteCount) || event.payload.byteCount < 0 || event.payload.byteCount > MAX_PAYLOAD_BYTES || !/^[A-Za-z0-9+/]*={0,2}$/.test(event.payload.value))) throw new Error('Activity ingestion rejected')
}

function bounded(value: unknown, max: number) {
  return typeof value === 'string' && value.length > 0 && value.length <= max && ![...value].some(character => character <= '\u001f' || character === '\u007f')
}

function boundedOptional(value: unknown, max: number) {
  return value === undefined || value === null || bounded(value, max)
}

function boundedMultilineOptional(value: unknown, max: number) {
  return value === undefined || value === null || (typeof value === 'string' && value.length > 0 && value.length <= max && ![...value].some(character => character < ' ' && !['\n', '\r', '\t'].includes(character)))
}

function transitionAllowed(from: ActivityStatus, to: ActivityStatus) {
  return from === 'started' ? true : from === 'running' ? to === 'running' || isTerminal(to) : from === to
}

function isTerminal(status: ActivityStatus) {
  return ['ok', 'error', 'denied', 'cancelled', 'interrupted'].includes(status)
}

function evidenceMetadata(event: ActivityIngressEvent) {
  const metadata = { evidence: event.presentation.evidence, complete: event.presentation.complete, action: event.presentation.action ?? undefined, summary: event.presentation.summary ?? undefined, resultDetail: event.presentation.resultDetail ?? undefined, payloadReference: event.presentation.payloadReference ?? undefined, affectedPaths: [] as string[], additions: 0, deletions: 0 }
  if (event.payload && event.presentation.evidence === 'exact') {
    let invalidPath = false
    try {
      const raw = Buffer.from(event.payload.value, 'base64').toString('utf8')
      const payload = JSON.parse(raw) as { files?: Array<{ path?: string, before?: string, after?: string }> }
      for (const file of payload.files ?? []) {
        const path = safeEvidencePath(file.path)
        if (!path) {
          invalidPath = true
          continue
        }
        metadata.affectedPaths.push(path)
        const counts = diffCounts(file.before ?? '', file.after ?? '')
        metadata.additions += counts.additions
        metadata.deletions += counts.deletions
      }
    } catch {
      metadata.complete = false
      metadata.evidence = 'unavailable'
    }
    if (invalidPath) {
      metadata.complete = false
      metadata.evidence = 'unavailable'
    }
  }
  return metadata
}

function mapItem(row: ActivityRow): ActivityItem {
  const evidence = (row.changeEvidence ?? {}) as { evidence?: ActivityEvidence, affectedPaths?: string[], additions?: number, deletions?: number, complete?: boolean, action?: string, summary?: string, resultDetail?: string }
  return {
    id: row.id,
    occurredAt: row.occurredAt.toISOString(),
    actor: { label: row.actor, source: row.actorSource ?? undefined, channel: row.channel },
    clientInfo: row.clientInfoName && row.clientInfoVersion ? { name: row.clientInfoName, version: row.clientInfoVersion } : undefined,
    operation: row.tool,
    category: row.category,
    effects: row.effects ?? [],
    target: row.target,
    action: evidence.action,
    status: row.status,
    durationMs: row.durationMs ?? undefined,
    affectedPaths: evidence.affectedPaths,
    additions: evidence.additions,
    deletions: evidence.deletions,
    evidence: evidence.evidence ?? 'not_applicable',
    result: evidence.summary,
    resultDetail: evidence.resultDetail,
    complete: evidence.complete ?? isTerminal(row.status),
    diffAvailable: evidence.evidence === 'exact' && Boolean(evidence.affectedPaths?.length)
  }
}

function activitySecretOrUndefined() {
  const secret = useRuntimeConfig().activityPayloadSecret
  return typeof secret === 'string' && secret.length >= 32 ? secret : undefined
}

function escapeLike(value: string) {
  return value.replace(/[\\%_]/g, '\\$&')
}

function safeEvidencePath(value: unknown) {
  if (typeof value !== 'string' || value.length === 0 || value.length > 512 || value.includes('\0') || value.startsWith('/') || /^[A-Za-z]:[\\/]/.test(value)) return undefined
  const segments = value.split(/[\\/]/)
  if (segments.some(segment => !segment || segment === '.' || segment === '..')) return undefined
  return segments.join('/')
}

function diffFile(path: string, before: string, after: string) {
  const oldLines = before ? before.split(/\r?\n/) : []
  const newLines = after ? after.split(/\r?\n/) : []
  let prefix = 0
  while (prefix < oldLines.length && prefix < newLines.length && oldLines[prefix] === newLines[prefix]) prefix++
  let suffix = 0
  while (suffix < oldLines.length - prefix && suffix < newLines.length - prefix && oldLines[oldLines.length - 1 - suffix] === newLines[newLines.length - 1 - suffix]) suffix++
  const removed = oldLines.slice(prefix, oldLines.length - suffix)
  const added = newLines.slice(prefix, newLines.length - suffix)
  const hunk = [`@@ -${prefix + 1},${removed.length} +${prefix + 1},${added.length} @@`, ...removed.map(line => `-${line}`), ...added.map(line => `+${line}`)]
  return { path, hunks: hunk, additions: added.length, deletions: removed.length }
}

function diffCounts(before: string, after: string) {
  const oldLines = before ? before.split(/\r?\n/) : []
  const newLines = after ? after.split(/\r?\n/) : []
  let prefix = 0
  while (prefix < oldLines.length && prefix < newLines.length && oldLines[prefix] === newLines[prefix]) prefix++
  let suffix = 0
  while (suffix < oldLines.length - prefix && suffix < newLines.length - prefix && oldLines[oldLines.length - 1 - suffix] === newLines[newLines.length - 1 - suffix]) suffix++
  return { additions: newLines.length - prefix - suffix, deletions: oldLines.length - prefix - suffix }
}
