// Plan 035 Phase 3 — single sanitization chokepoint for server-authored log
// attributes/messages. Every server log call site (existing and future)
// flows through `logger.ts`'s `emit()`, which calls into this module before
// handing anything to the OTel Logs API / Loki exporter. This is an
// allowlist, not a denylist: unknown attribute keys are dropped, not passed
// through — per Plan 035's "allowlist beats denylist" rule and the frozen
// contract at .agents/contracts/035-observability-telemetry-contract.md §2.1.
//
// Deliberately a single function/module, not a generic
// `TelemetrySanitizer<T>`/factory — see the plan's anti-overengineering rule.

// §2.1 stable attribute vocabulary, plus two additions justified by Phase 3's
// explicit scope (see plan Phase 3 items 5-6): `error.message` (bounded,
// private-log-only diagnostic text — never returned to clients per Phase 2's
// sanitized error split) and `stack` (gated by `shouldIncludeStack()` below,
// never public). `trace_id`/`span_id` are correlation fields already living
// in the structured body per §1.4/§2.1, added here so the allowlist covers
// what `logger.ts` actually attaches.
const ALLOWED_ATTRIBUTE_KEYS = new Set<string>([
  'service.name',
  'deployment.environment',
  'component',
  'layer',
  'event.name',
  'operation',
  'outcome',
  'request.id',
  'http.request.method',
  'route',
  'http.response.status_code',
  'duration_ms',
  'error.type',
  'error.code',
  'error.message',
  'error.classification',
  'stack',
  'provider.type',
  'tool.name',
  'tool.id',
  'tool.category',
  'tool.effects',
  'policy.outcome',
  'policy.source',
  'result.classification',
  'result.truncated',
  'agent.profile',
  'agent.state',
  'agent.depth',
  'agent.tool_calls',
  'agent.output_tokens',
  'background.isolation',
  'background.state',
  'orchestration.run_id',
  'orchestration.node_id',
  'orchestration.role',
  'orchestration.state',
  'orchestration.readiness',
  'orchestration.result_class',
  'orchestration.worktree_lifecycle',
  'orchestration.reconciliation_outcome',
  'orchestration.started.count',
  'orchestration.denied.count',
  'orchestration.ready.count',
  'orchestration.running.count',
  'orchestration.issue.count',
  'orchestration.blocker.count',
  'orchestration.dependency.count',
  'orchestration.concurrency.slot',
  'context.pressure',
  'cancel.reason',
  'mcp.method',
  'attempt',
  'retry_count',
  'auth.present',
  'trace_id',
  'span_id'
])

// §6: "Max attribute value string length: 512 characters" / "Max attribute
// count per log record: 20" for server-side structured logs.
export const MAX_ATTRIBUTE_VALUE_LENGTH = 512
export const MAX_ATTRIBUTE_COUNT = 20
const MAX_MESSAGE_LENGTH = 1024
const MAX_ROUTE_LENGTH = 256

// Routes are structured classification attributes, not free text. Keep the
// route exception narrow and validate it here at the logger chokepoint so
// callers cannot accidentally reintroduce query strings or filesystem/URL
// content by bypassing request-lifecycle.ts.
const SAFE_ROUTE_PATTERN = /^\/[A-Za-z0-9._:/*-]*$/

export function sanitizeRoute(value: string): string {
  const withoutQuery = (value.split('?')[0] || '/').slice(0, MAX_ROUTE_LENGTH)
  return SAFE_ROUTE_PATTERN.test(withoutQuery) ? withoutQuery : '/'
}

// Strips C0/C1 control characters (except none — logs are single JSON lines,
// so even \n/\t are stripped) to keep stderr/console/Loki JSON lines safe
// and to prevent log-injection via attacker-influenced strings ending up in
// error messages.
function stripControlChars(value: string): string {
  // eslint-disable-next-line no-control-regex
  return value.replace(/[\x00-\x1f\x7f]/g, ' ')
}

// P1 value-level secret redaction (Plan 035 remediation round 2). Key
// allowlisting above only controls which attribute KEYS survive — it does
// not inspect the VALUES of allowed keys (e.g. `error.message`, `stack`)
// for embedded credential-shaped text. These patterns mask only the
// credential-shaped substring, keeping surrounding context readable for
// cause-classification. Deterministic regex only — no AI/heuristic
// detection, per the plan's explicit KISS instruction. Order matters: URL
// userinfo and Bearer/Basic are matched before the generic
// key=value/JWT passes so nothing double-matches oddly.
const FILESYSTEM_PATH_PATTERN: readonly [RegExp, string] = [
  /\/(?:[A-Za-z0-9._-]+\/){2,}[A-Za-z0-9._-]*/g,
  '[REDACTED-PATH]'
]

// Keys whose values are already structured/validated elsewhere (never raw
// free text) and must NOT go through `FILESYSTEM_PATH_PATTERN`: that
// pattern is tuned for error messages/stacks and would otherwise collapse a
// normal multi-segment API route like `/api/auth/register` down to
// `[REDACTED-PATH]`. `route` is validated/normalized by `safeRoute()` in
// `request-lifecycle.ts` before it ever reaches this module (query string
// stripped, length-capped, charset-checked) — see that module for the
// dedicated route sanitizer. Other §2.1 attribute keys are short enum-like
// tokens (e.g. `operation`, `event.name`) with no slashes in practice, so
// they are unaffected by the generic pattern and don't need this bypass.
const SKIP_FILESYSTEM_PATH_REDACTION = new Set<string>(['route'])

const REDACTION_PATTERNS: ReadonlyArray<readonly [RegExp, string]> = [
  // Absolute filesystem paths (including source-map stack locations). Paths
  // are sensitive workspace/environment data and are not useful as a stable
  // exception classification.
  FILESYSTEM_PATH_PATTERN,
  // URL userinfo credentials, incl. DB connection strings:
  // postgres://user:pass@host -> postgres://[REDACTED]@host
  [/([a-zA-Z][a-zA-Z0-9+.-]*):\/\/[^/\s:@]+:[^/\s:@]+@/g, '$1://[REDACTED]@'],
  // Bearer <token>
  [/\bBearer\s+[A-Za-z0-9\-._~+/]+=*/gi, 'Bearer [REDACTED]'],
  // Basic <base64>
  [/\bBasic\s+[A-Za-z0-9+/]+=*/gi, 'Basic [REDACTED]'],
  // Header/assignment-style credentials: x-api-key=, apikey:, cookie=,
  // session=, password=, token=, secret=, key= (and separator variants),
  // including JSON-shaped `"apiKey":"value"` where the key itself is
  // quoted before the `:`/`=` separator.
  [
    /\b(x-api-key|api[-_]?key|apikey|cookie|session|password|passwd|token|secret|access[-_]?key|client[-_]?secret|key)['"]?\s*[:=]\s*['"]?[^\s'",;}]+/gi,
    '$1=[REDACTED]'
  ],
  // JWT-like values: header.payload.signature, base64url, header starts eyJ
  [/\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b/g, '[REDACTED-JWT]']
]

/**
 * Masks credential-shaped substrings (bearer/basic auth, API keys,
 * cookies/sessions, DB/URL userinfo, password/token/secret/key
 * assignments, JWT-like tokens) inside an arbitrary string value. Never
 * throws — falls back to the original string on any regex failure, same
 * defensive posture as `sanitizeValue`'s JSON.stringify fallback. Bounded:
 * only substitutes matched substrings, never expands the string.
 */
export function redactSecrets(value: string, attributeKey?: string): string {
  try {
    let out = value
    const skipFilesystemPath = attributeKey !== undefined && SKIP_FILESYSTEM_PATH_REDACTION.has(attributeKey)
    for (const [pattern, replacement] of REDACTION_PATTERNS) {
      if (skipFilesystemPath && pattern === FILESYSTEM_PATH_PATTERN[0]) continue
      out = out.replace(pattern, replacement)
    }
    return out
  } catch {
    return value
  }
}

function sanitizeString(value: string, maxLength: number, attributeKey?: string): string {
  const cleaned = stripControlChars(redactSecrets(value, attributeKey)).trim()
  return cleaned.length > maxLength ? `${cleaned.slice(0, maxLength)}…` : cleaned
}

function sanitizeValue(value: unknown, attributeKey?: string): string | number | boolean | undefined {
  if (value === null || value === undefined) return undefined
  if (attributeKey === 'route' && typeof value === 'string') return sanitizeRoute(value)
  if (typeof value === 'string') return sanitizeString(value, MAX_ATTRIBUTE_VALUE_LENGTH, attributeKey)
  if (typeof value === 'number' || typeof value === 'boolean') return value
  // Any other shape (object/array/function/symbol) is not part of the
  // vocabulary's allowed value shapes — stringify defensively and cap, or
  // drop entirely if it can't be serialized (e.g. circular structures),
  // rather than ever throwing from the logging path.
  try {
    return sanitizeString(JSON.stringify(value), MAX_ATTRIBUTE_VALUE_LENGTH, attributeKey)
  } catch {
    return undefined
  }
}

/**
 * Allowlist-filters, length-caps, and control-character-strips a raw
 * attributes bag before it reaches the OTel Logs API / Loki exporter.
 * Unknown keys are dropped silently (never passed through). Bounded to
 * `MAX_ATTRIBUTE_COUNT` entries, keeping insertion order.
 */
export function sanitizeAttributes(attributes: Record<string, unknown>): Record<string, string | number | boolean> {
  const out: Record<string, string | number | boolean> = {}
  let count = 0
  for (const [key, raw] of Object.entries(attributes)) {
    if (count >= MAX_ATTRIBUTE_COUNT) break
    if (!ALLOWED_ATTRIBUTE_KEYS.has(key)) continue
    const value = sanitizeValue(raw, key)
    if (value === undefined) continue
    out[key] = value
    count += 1
  }
  return out
}

/** Bounds and cleans the human-readable log message/body string. */
export function sanitizeMessage(message: string): string {
  return sanitizeString(message, MAX_MESSAGE_LENGTH)
}

/**
 * Stack-trace inclusion policy (Plan 035 Phase 3 item 6): development
 * environments include stacks by default for debuggability; production
 * requires an explicit opt-in env flag. Either way, the stack only ever
 * reaches the structured private log body (via the `stack` attribute,
 * allowlisted above) — it is never part of any client-visible response
 * (guaranteed independently by Phase 2's `problem()`/error-handler split).
 */
export function shouldIncludeStack(): boolean {
  if (process.env.NODE_ENV !== 'production') return true
  return process.env.NUXT_OTEL_LOG_STACK_TRACES === 'true'
}
