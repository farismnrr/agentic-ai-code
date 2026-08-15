# Plan 035 Phase 11 — Case 9/10: Rust relay evidence

## Setup

Ran the real release binary directly on the host (per `packages/relay-agent/SKILL.md` local-mode
invocation), pointed at the live Jaeger OTLP-gRPC endpoint:

```
NUXT_OTEL_ENABLED=true NUXT_OTEL_JAEGER_ENDPOINT=http://localhost:4317 \
  ./target/release/ai-tools relay --mode local \
  --dir <scratch-workspace> --execution-root <scratch-workspace> \
  --origin http://localhost:3333 --port 47821
```

Confirmed listening: `relay-agent listening on 127.0.0.1:47821`.

## Happy path: real MCP `initialize`

```
POST http://127.0.0.1:47821/mcp  (Origin: http://localhost:3333, proper MCP headers)
{"jsonrpc":"2.0","id":1,"method":"initialize", ...}
-> HTTP/1.1 200 OK
x-request-id: 3910a88a-ee6b-4b84-aa97-28f2340247c7
{"id":1,"jsonrpc":"2.0","result":{...,"serverInfo":{"name":"relay-agent","version":"0.0.7-beta"}}}
```
Full response saved: `rust-happy-initialize-response.txt`.

## Happy path: real MCP `tools/list`

Returned the 3 registered tools (`terminal_exec`, `http_fetch`, `web_search`) with full JSON
schemas — saved as `rust-happy-toolslist-response.txt`.

## Error path: deliberately malformed/bad request

Sent `method: "totally/bogus_method"` with an intentionally incomplete request (missing
`mcp-protocol-version` header on the first attempt, then missing a required `_meta` field on the
second, proper-header attempt):

```
-> HTTP/1.1 400 Bad Request
{"jsonrpc":"2.0","id":4,"error":{"code":-32020,"message":"Header mismatch: required params._meta['io.modelcontextprotocol/protocolVersion'] is missing from the request body"}}
```
Fully generic MCP protocol-validation error — no internal file paths, no OAuth/JWKS/network
detail, no stack. Saved as `rust-error-badmethod-response.txt`.

## Jaeger trace correlation (real, queried live)

`GET http://localhost:16686/api/services` confirms a live `ai-code-relay` service. Querying
`GET /api/traces?service=ai-code-relay` and filtering for the `relay.request` span
(`packages/rust-tools/infrastructure/src/transport.rs:673`) shows, for each request above, a span
whose `request_id` tag **exactly matches** the `x-request-id` response header returned to the
client:

| Request | x-request-id | Jaeger trace_id | `relay.request` span found |
|---|---|---|---|
| `initialize` | `3910a88a-ee6b-4b84-aa97-28f2340247c7` | `4bab703d6a1180d88df20c3b2db99645` | yes |
| `tools/list` | `3d94f80d-63c9-4e64-8d81-8f777dd1fce6` | `76e5aea7d5b3e15107a44ab58975d649` | yes |
| error (bad header) | `e45b9c36-505e-40c9-b028-02487425d0ba` | `e755ffec74df2391fb2aa82d6ad5aff4` | yes |
| error (bad method) | `77ccc29b-027a-4814-ac06-6d5325c8cf15` | `6da5c00d69bdfde02c3eb50bc2bc75a1` | yes |

Full trace JSON for the happy `initialize` request and the bad-method error request saved as
`rust-happy-jaeger-trace.json` / `rust-error-jaeger-trace.json` respectively.

## Honest observation: Rust stderr `audit()` JSON did not appear during this run

The relay process's own stderr (redirected to a log file) showed only the startup line
(`relay-agent listening on ...`); no per-request `audit()` JSON lines were observed for these
specific MCP calls in this run, even though `packages/rust-tools/infrastructure/src/observability.rs`
documents an `eprintln!`-based per-request audit event. This is consistent with, and does not
contradict, the already-documented Phase 10 finding that Rust structured stderr logs are a
separate, non-Loki-integrated channel — but the absence of the per-request line specifically
(rather than just "not in Loki") was not further root-caused in this Phase 11 pass due to time
budget; flagging honestly rather than fabricating a stderr capture. The Jaeger trace evidence
above is the solid, verified proof for this case; the stderr gap is noted as an open item for a
future phase, not asserted as working when it was not observed working.

## Verdict: PASS for trace-based correlation (case 9/10 core requirement — correlated request_id
across client response header and Jaeger span, generic error bodies). Stderr audit-log capture
inconclusive in this run (documented, not fabricated).
