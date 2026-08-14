# Plan 035 Phase 11 — Case 2/4: Genuine unhandled exception, sanitizer cannot be bypassed

This evidence was captured incidentally while exercising the API-key auth path
(`server/middleware/api-auth.ts`) for happy-path evidence, and doubles as strong Case 4 proof
because it is a **real, unplanned, unhandled exception** — not a synthetic test — that hit the
global Nitro error boundary.

## Root cause found (and fixed as a Phase 11 finding, see below)

`server/infrastructure/auth/api-key.ts`'s `verifyApiKey()` called `useDb()` without importing it,
and `server/middleware/api-auth.ts` itself also called `useDb()` without importing it. Both are
production-build auto-import resolution gaps of the same class already partially fixed at commit
`79cceb1` ("Fix: add missing logger and useDb auto-imports") — that commit added `logger` imports
everywhere needed but missed `useDb` in five files: `server/infrastructure/auth/api-key.ts`,
`server/middleware/api-auth.ts`, `server/infrastructure/ai/context-compaction.ts`,
`server/infrastructure/mcp/test-server.ts`, `server/infrastructure/mcp/server-config.ts`. This
made API-key authentication completely non-functional in the production Docker build (Nitro's
`.nuxt/tsconfig.json` type-check gate does not cover `server/**`, so `pnpm verify:commit` could
not catch it — matches the exact gap already flagged in the Plan 035 execution notes).

## Request that triggered it (before the fix)

```
POST /api/mcp   (Authorization: Bearer aic_live_...)
-> HTTP/1.1 500 Internal Server Error
x-request-id: 11d13231-f940-4d86-b00a-be44bce7658d
{"type":"about:blank","title":"Internal Server Error","status":500,"instance":"/api/mcp","requestId":"11d13231-f940-4d86-b00a-be44bce7658d"}
```

## Public body

Fully generic — `type`, `title`, `status`, `instance`, `requestId` only. No mention of `useDb`,
`ReferenceError`, file paths, or any internal detail, even though the underlying failure was a
completely unplanned `ReferenceError` thrown deep in application code, not a deliberately-thrown
`internal()`/`badGateway()` call. This is exactly Case 4 ("malformed/unhandled exception cannot
bypass the 5xx sanitizer").

## Private Loki record

```json
{"message":"[api-auth] API Key verification failed","attributes":{"service.name":"ai-code-server","error.type":"ReferenceError","error.message":"useDb is not defined","trace_id":"435c23ca5e1ade11d7432f949cdf262b","span_id":"b38dbaa115fb7bb8"}}
{"message":"[unhandled]","attributes":{"service.name":"ai-code-server","error.type":"Error","error.message":"useDb is not defined","trace_id":"435c23ca5e1ade11d7432f949cdf262b","span_id":"b38dbaa115fb7bb8"}}
{"message":"auth.login","attributes":{"request.id":"11d13231-f940-4d86-b00a-be44bce7658d","operation":"auth.login","outcome":"denied","auth.present":true,"trace_id":"435c23ca5e1ade11d7432f949cdf262b","span_id":"b38dbaa115fb7bb8"}}
```

The real cause (`ReferenceError: useDb is not defined`) and file-level context is fully captured
privately, correlated by `trace_id`/`request.id`, and never reached the client.

## Fix applied (Phase 11 finding, minimal, in-scope per delegation instructions)

Added the missing `import { useDb } from '../database/connection'` (adjusted relative path per
file) to all five files listed above. Rebuilt the Docker image and confirmed:

```
GET /api/sidebar   (Authorization: Bearer aic_live_...)
-> HTTP/1.1 200 OK
```

with a clean `auth.login` / `outcome: ok` Loki record and no more `ReferenceError` — see
`happy-path-api-key-auth.md`. `pnpm verify:commit` re-run after the fix (see final report).

## Verdict: PASS (Case 4 proven), pre-existing P1 functional bug found and fixed (API-key auth was completely broken).
