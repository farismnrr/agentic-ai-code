export type ContextUsageKind = 'provider_measured_boundary' | 'estimated_from_provider_boundary' | 'unknown'

export interface ContextInspectorData {
  contextWindow: number | null
  usedTokens: number | null
  usedTokensKind: ContextUsageKind
  reservedOutputTokens: number | null
  headroom: number | null
  summaryPresent: boolean
  summaryAgeMs: number | null
  activeChildren: number | null
  activeBackgroundTasks: number | null
  pressure: boolean | 'unknown'
}

export interface ContextUsagePresentation {
  state: 'measured' | 'estimated' | 'unavailable'
  percent: number | null
  label: string
  detail: string | null
}

/** Present only bounded inspector metadata; never render summaries or provider payloads. */
export function presentContextUsage(data: ContextInspectorData | null): ContextUsagePresentation {
  if (!data || data.usedTokens == null || data.contextWindow == null || data.usedTokensKind === 'unknown') {
    return { state: 'unavailable', percent: null, label: 'Context unavailable', detail: null }
  }

  const usableContext = data.contextWindow - (data.reservedOutputTokens ?? 0)
  if (usableContext <= 0) {
    return { state: 'unavailable', percent: null, label: 'Context unavailable', detail: null }
  }

  const percent = Math.min(100, Math.max(0, Math.round((data.usedTokens / usableContext) * 100)))
  const state = data.usedTokensKind === 'provider_measured_boundary' ? 'measured' : 'estimated'
  const label = state === 'measured' ? `${percent}% measured boundary` : `${percent}% estimated`
  const detail = data.headroom == null ? null : `${data.headroom.toLocaleString()} tokens available`
  return { state, percent, label, detail }
}
