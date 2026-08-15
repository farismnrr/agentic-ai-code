# Plan 035 live shared-observability closure evidence

Date: 2026-08-15
Branch: `feat/035-p0-observability-contract`
Final implementation commit: `10f5dfd6b93f7e0bae9619005e81af4b5dc1abcf`

## Existing infrastructure

- App container: `ai-code-app-1`
- Docker networks: `masihawam-net`, `shared-network`
- Jaeger query API: host port `16686`
- Jaeger OTLP gRPC: app-network endpoint `jaeger:4317`
- Loki query API: host port `3101`
- Loki push API: app-network endpoint `loki:3100/loki/api/v1/push`

The existing shared containers were used as-is. No duplicate observability
stack was created and no shared container was stopped or recreated.

## Live proofs

- `scripts/verify-phase2-route-telemetry.sh`: PASS against the rebuilt app,
  shared Loki, and shared Jaeger. Static route was exactly
  `/api/auth/register`; dynamic and unmatched routes used only the approved
  stable template/coarse fallback; dynamic IDs, unmatched canary, and query
  canary were absent. `x-request-id -> Loki request.id -> trace_id -> Jaeger`
  correlation resolved live.
- `node scripts/verify-phase4-raw-error-canary.mjs`: PASS against the shared
  Loki and Jaeger APIs. Mutable raw `Error.name`, raw message, and raw stack/path
  canaries were absent from stdout, Loki, and Jaeger; bounded error type and
  classification assertions passed.
- `pnpm verify:commit`, `pnpm build`, `pnpm audit`, `cargo audit`, current
  deterministic Plan 035 checks, and server/LSP review: PASS as reported by the
  implementation worker.
- Fresh independent closure review: PASS; zero unresolved P0/P1.

No credentials, cookies, bearer tokens, session data, or real user PII are
recorded here. No executable PII canary harness exists in the current tree;
the separate committed live PII evidence remains
`phase2-pii-canary-proof.md`, and its absence is not claimed as a newly rerun
PASS.
