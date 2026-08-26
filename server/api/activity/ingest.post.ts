import * as v from 'valibot'
import { badRequest, forbidden } from '#server/core/errors/http'

const presentationSchema = v.strictObject({
  target: v.optional(v.nullable(v.pipe(v.string(), v.maxLength(4096)))),
  summary: v.optional(v.nullable(v.pipe(v.string(), v.maxLength(256)))),
  result_class: v.optional(v.nullable(v.pipe(v.string(), v.maxLength(256)))),
  evidence: v.picklist(['exact', 'summary', 'unavailable', 'not_applicable']),
  payload_reference: v.optional(v.nullable(v.pipe(v.string(), v.maxLength(256)))),
  complete: v.boolean()
})

const eventSchema = v.strictObject({
  recordId: v.pipe(v.string(), v.minLength(1), v.maxLength(320)),
  event: v.strictObject({
    contract_version: v.literal('activity.event.v1'),
    activity_id: v.pipe(v.string(), v.minLength(1), v.maxLength(256)),
    source_id: v.pipe(v.string(), v.minLength(1), v.maxLength(256)),
    source_sequence: v.pipe(v.number(), v.safeInteger(), v.minValue(1), v.maxValue(2147483647)),
    status: v.picklist(['started', 'running', 'ok', 'error', 'denied', 'cancelled', 'interrupted']),
    tool_id: v.pipe(v.string(), v.minLength(1), v.maxLength(256)),
    category: v.picklist(['filesystem', 'search', 'terminal', 'git', 'code', 'delegated', 'network', 'workspace', 'other']),
    effects: v.pipe(v.array(v.picklist(['workspace_read', 'workspace_write', 'workspace_delete', 'process_exec', 'network_read', 'network_write', 'git_read', 'external_mutation', 'privileged_bridge'])), v.maxLength(64)),
    workspace_root_fingerprint: v.optional(v.nullable(v.pipe(v.string(), v.regex(/^[a-f0-9]{64}$/)))),
    actor: v.strictObject({
      label: v.pipe(v.string(), v.maxLength(256)),
      source: v.optional(v.nullable(v.pipe(v.string(), v.maxLength(256)))),
      channel: v.optional(v.nullable(v.pipe(v.string(), v.maxLength(256))))
    }),
    client_info: v.optional(v.nullable(v.strictObject({
      name: v.pipe(v.string(), v.minLength(1), v.maxLength(256)),
      version: v.pipe(v.string(), v.minLength(1), v.maxLength(64))
    }))),
    occurred_at_ms: v.pipe(v.number(), v.safeInteger()),
    duration_ms: v.optional(v.nullable(v.pipe(v.number(), v.safeInteger(), v.minValue(0), v.maxValue(2147483647)))),
    presentation: presentationSchema
  }),
  payload: v.optional(v.strictObject({
    kind: v.pipe(v.string(), v.minLength(1), v.maxLength(40)),
    version: v.pipe(v.string(), v.minLength(1), v.maxLength(24)),
    value: v.pipe(v.string(), v.maxLength(699052), v.regex(v.BASE64_REGEX)),
    byteCount: v.pipe(v.number(), v.safeInteger(), v.minValue(0), v.maxValue(524288))
  }))
})

const batchSchema = v.strictObject({
  contractVersion: v.literal('activity.event.v1'),
  sourceId: v.pipe(v.string(), v.minLength(1), v.maxLength(256)),
  events: v.pipe(v.array(eventSchema), v.minLength(1), v.maxLength(100))
})

const MAX_BATCH_BYTES = 4 * 1024 * 1024

async function readBoundedJson(request: AsyncIterable<Uint8Array>) {
  const chunks: Uint8Array[] = []
  let byteCount = 0
  for await (const chunk of request) {
    byteCount += chunk.byteLength
    if (byteCount > MAX_BATCH_BYTES) throw badRequest('Activity batch is too large')
    chunks.push(chunk)
  }
  try {
    return JSON.parse(Buffer.concat(chunks).toString('utf8')) as unknown
  } catch {
    return undefined
  }
}

export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'Cache-Control', 'no-store')
  const contentType = getHeader(event, 'content-type')?.split(';', 1)[0]
  if (contentType !== 'application/json') throw badRequest('Activity batches must use JSON')
  const contentLength = Number(getHeader(event, 'content-length'))
  if (Number.isFinite(contentLength) && contentLength > MAX_BATCH_BYTES) throw badRequest('Activity batch is too large')
  const authorization = getHeader(event, 'Authorization')
  if (!authorization?.startsWith('Bearer ')) throw forbidden('Activity source credential is required')
  const parsed = v.safeParse(batchSchema, await readBoundedJson(event.node.req))
  if (!parsed.success) throw badRequest('Activity batch is invalid')
  const body = parsed.output
  if (body.events.some(item => item.event.source_id !== body.sourceId)) throw badRequest('Activity source identity is inconsistent')
  try {
    return await event.context.application.activity.ingest(authorization.slice(7), body.events.map(item => ({
      recordId: item.recordId,
      sourceId: item.event.source_id,
      contractVersion: item.event.contract_version,
      activityId: item.event.activity_id,
      sourceSequence: item.event.source_sequence,
      status: item.event.status,
      toolId: item.event.tool_id,
      category: item.event.category,
      effects: item.event.effects,
      workspaceRootFingerprint: item.event.workspace_root_fingerprint,
      actor: item.event.actor,
      clientInfo: item.event.client_info,
      occurredAtMs: item.event.occurred_at_ms,
      durationMs: item.event.duration_ms,
      presentation: {
        target: item.event.presentation.target,
        summary: item.event.presentation.summary,
        resultClass: item.event.presentation.result_class,
        evidence: item.event.presentation.evidence,
        payloadReference: item.event.presentation.payload_reference,
        complete: item.event.presentation.complete
      },
      payload: item.payload
    })))
  } catch (error) {
    if (error instanceof Error && error.message === 'Activity ingestion rejected') throw forbidden('Activity source credential or workspace binding is invalid')
    if (error instanceof Error && (/^Activity (batch limit|lifecycle transition|payload|ingestion) /.test(error.message) || error.message === 'Activity ingestion failed')) throw badRequest('Activity batch cannot be accepted')
    throw error
  }
})
