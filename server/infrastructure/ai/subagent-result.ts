import type { BackgroundTaskMetadata, SubagentResult } from '#shared/types/subagents'
import { redactSecrets } from '../observability/sanitize.ts'

const MAX_SUMMARY = 4096
const MAX_ITEM = 1024
const MAX_ITEMS = 32

type SafeModelResult = Pick<SubagentResult, 'status' | 'summary' | 'findings' | 'evidence' | 'validation' | 'remaining_risks'>

function redactPresentationPaths(value: string): string {
  return value
    .replace(/\\\\[^\s"'`<>]+(?:\\[^\s"'`<>]+)*/g, '[REDACTED-PATH]')
    .replace(/\b[A-Za-z]:[\\/][^\s"'`<>]*/g, '[REDACTED-PATH]')
    .replace(/(^|[^A-Za-z0-9+.-])\/(?!\/)[^\s"'`<>]*/g, '$1[REDACTED-PATH]')
}

function safeText(value: unknown, limit: number): string | undefined {
  if (typeof value !== 'string') return undefined
  const cleaned = redactPresentationPaths(redactSecrets(value)).replaceAll(/\p{Cc}/gu, ' ').trim()
  return cleaned ? cleaned.slice(0, limit) : undefined
}

function safeList(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return value.slice(0, MAX_ITEMS).flatMap((item) => {
    const text = safeText(item, MAX_ITEM)
    return text ? [text] : []
  })
}

function safeEvidence(value: unknown): Array<{ reference: string, detail: string }> {
  if (!Array.isArray(value)) return []
  return value.slice(0, MAX_ITEMS).flatMap((item) => {
    if (typeof item !== 'object' || item === null) return []
    const reference = safeText((item as { reference?: unknown }).reference, MAX_ITEM)
    const detail = safeText((item as { detail?: unknown }).detail, MAX_ITEM)
    return reference && detail ? [{ reference, detail }] : []
  })
}

/** Model output is untrusted. Only strict structured summary fields survive, with deterministic redaction and bounds. */
export function parsePresentationSafeSubagentResult(text: string): SafeModelResult | undefined {
  let raw: unknown
  try {
    raw = JSON.parse(text)
  } catch {
    return undefined
  }
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) return undefined
  const value = raw as Record<string, unknown>
  const summary = safeText(value.summary, MAX_SUMMARY)
  if (!summary) return undefined
  const allowedStatus = typeof value.status === 'string' && ['completed', 'blocked', 'cancelled', 'budget_exhausted', 'failed', 'invalid'].includes(value.status) ? value.status as SubagentResult['status'] : 'invalid'
  return { status: allowedStatus, summary, findings: safeList(value.findings), evidence: safeEvidence(value.evidence), validation: safeList(value.validation), remaining_risks: safeList(value.remaining_risks) }
}

function presentationSafeResult(result: SubagentResult): SubagentResult {
  return {
    ...result,
    summary: safeText(result.summary, MAX_SUMMARY) ?? 'Child returned no presentation-safe summary.',
    findings: safeList(result.findings),
    evidence: safeEvidence(result.evidence),
    validation: safeList(result.validation),
    remaining_risks: safeList(result.remaining_risks)
  }
}

/** Background metadata crosses the application→tool/UI boundary here; sanitize evidence appended after child execution. */
export function presentationSafeBackgroundTask(task: BackgroundTaskMetadata): BackgroundTaskMetadata {
  return {
    ...task,
    repository_identity: '[repository]',
    progress_summary: safeText(task.progress_summary, MAX_ITEM) ?? 'Background task status unavailable.',
    worktree_path: task.worktree_path ? '[isolated-worktree]' : undefined,
    result: task.result ? presentationSafeResult(task.result) : undefined
  }
}
