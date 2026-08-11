# Plan 028 — Phase 0 contract + MCP freeze audit

Source of truth read in full: `packages/relay-agent/src/server.ts`, `src/index.ts`,
`bin/cli.mjs`, `bin/pidfile.mjs`, `package.json`; Nuxt consumer
`app/composables/useRelayAgent.ts` (and its callers `app/pages/settings/local-terminal.vue`,
`server/api/chat.post.ts`, `shared/utils/native-tools.ts`).

## 1. Legacy HTTP/WS contract (frozen, for Phase 4 — not built this run)

### CLI (`bin/cli.mjs`)

- Flags: `--port`/`-p` (default `47821`), `--dir`/`-d` (no CLI default; server defaults to
  `os.homedir()`), `--origin`/`-o` (no CLI default; falls back to `process.env.RELAY_AGENT_ORIGIN`;
  server itself defaults to `http://localhost:3333` only if neither is given).
- Positional `stop`: reads the port-scoped pidfile, sends `SIGTERM` to a live pid, exits 0 whether
  or not anything was running (stale pidfile is silently cleared, not an error).
- Refuses to start a second instance on the same port (`acquireLock`), printing the existing pid
  and exiting 1.
- `SIGINT`/`SIGTERM` → clean shutdown: remove pidfile if owned by this pid, close server, exit 0.

### Pidfile (`bin/pidfile.mjs`)

- Location: `$XDG_RUNTIME_DIR/relay-agent-<port>.pid` if set, else `~/.relay-agent/relay-agent-<port>.pid`.
- Acquisition: exclusive `open(path, 'wx')` (atomic create-if-not-exists) containing the raw pid.
  On `EEXIST`, check `isProcessAlive` (via `kill(pid, 0)`); if dead, delete and retry once; if
  alive, refuse.
- Only ever deletes a pidfile if it currently contains this process's own pid (`removePidFileIfOwnedByMe`).

### HTTP endpoints (all require Host + Origin validation first — see below)

- `GET /health` → `200`, `Content-Type: application/json`,
  body `{"status":"ok","agent":"relay-agent","defaultCwd":"<resolved dir>"}`.
- `POST /pair` → body `{"token": "<pairing token>"}`.
  - Missing/mismatched token → `401 {"error":"Invalid pairing token"}`.
  - Expired (TTL 5 min from process start) → `401 {"error":"Pairing token has expired (~5 min TTL)"}`.
  - Success → `200 {"sessionCredential":"<64 hex chars>"}`; pairing token is single-use — cleared
    (`""`) immediately after the first successful use, so a replay is a mismatch, not a second success.
  - Malformed JSON body → `400 {"error":"<message>"}`.
- `POST /revoke` → body `{"credential": "<session credential>"}`.
  - Known credential → `200 {"success":true,"message":"Session credential revoked"}`, credential removed from the in-memory set.
  - Unknown/missing → `400 {"error":"Invalid credential"}`.
- `OPTIONS *` → `204`, no body, with the CORS headers below already set.
- Any other path/method → `404 {"error":"Not found"}`.
- CORS headers set on every non-rejected response: `Access-Control-Allow-Origin: <allowedOrigin>`
  (never `*`), `Access-Control-Allow-Methods: GET, POST, OPTIONS`,
  `Access-Control-Allow-Headers: Content-Type, Authorization`.

### Host/Origin validation (applied to every HTTP request and every WS upgrade)

- `Host` header must be exactly `127.0.0.1:<port>` or `localhost:<port>`; anything else (including
  missing) → `403 {"error":"Invalid Host header: <host>"}`.
- `Origin` header must be present and exactly equal `allowedOrigin`; anything else (including
  missing, and including a wildcard config value) → `403 {"error":"Disallowed or missing Origin header: <origin>"}`.

### WebSocket

- Same origin/host check on the `upgrade` event before handoff; failure → raw
  `HTTP/1.1 403 Forbidden\r\n\r\n` and destroy the socket (no JSON body — this happens pre-HTTP-response).
- Upgrade request must carry `?credential=<sessionCredential>` matching a currently-paired
  credential; otherwise raw `HTTP/1.1 401 Unauthorized\r\n\r\n` and destroy the socket. Pairing
  tokens are never accepted here — only session credentials minted by `/pair`.
- Path/query used by the Nuxt client: `ws://127.0.0.1:<port>?credential=<cred>` (no distinct path,
  root path only).
- Message in — `exec`: `{"type":"exec","id":"<client-chosen id>","command":"<string>","args":["..."],"cwd":"<optional>"}`.
  - `command` is split on whitespace into `[binary, ...gluedArgs]`; `finalArgs = gluedArgs ++ args`.
    No shell is invoked (`execa(..., { shell: false })`).
  - `cwd`: `path.resolve(defaultCwd, cwd)` if given (absolute `cwd` short-circuits to itself, no
    directory jail — documented as deliberate in `server.ts`), else `defaultCwd`.
  - Env passed to the child is reduced to `PATH`, `HOME`, `LANG` only (`extendEnv: false`).
  - Timeout: `execTimeoutMs` (default 300000ms / 5 min), `killSignal: SIGKILL`.
- Message out — `exec_result`: always `{"type":"exec_result","id"}` plus one of:
  - timeout: `{success:false, error:"Command timed out after <n>s", stdout, stderr}` (no exitCode).
  - non-zero/failed: `{success:false, error:<shortMessage>, exitCode, stdout, stderr}`.
  - success: `{success:true, exitCode, stdout, stderr}`.
  - thrown before spawn (e.g. missing `command`): `{success:false, error:<message>}` (no stdout/stderr/exitCode).
- Unknown message `type` → `{"type":"error","error":"Unknown message type: <type>"}`.
- Malformed JSON on the socket → `{"type":"error","error":"<parse error message>"}`.
- The client backstops with a 310s client-side timeout independent of the server's 300s exec timeout.

### Env var precedence

`--origin`/`-o` (CLI flag) wins over `RELAY_AGENT_ORIGIN` (env) wins over the server's own
`http://localhost:3333` default. `--dir`/`-d` wins over `os.homedir()`; there is no dir env var.

## 2. Decision: does Nuxt need the legacy compat layer?

**Yes — confirmed required for a later phase (Phase 4), not eliminable in this rewrite.**
`app/composables/useRelayAgent.ts` (consumed by `app/pages/settings/local-terminal.vue`,
`server/api/chat.post.ts` via `shared/utils/native-tools.ts`) talks to the relay agent using:

- `$fetch('http://127.0.0.1:<port>/pair', { method: 'POST', body: { token } })`
- `$fetch('http://127.0.0.1:<port>/revoke', { method: 'POST', body: { credential } })`
- `new WebSocket('ws://127.0.0.1:<port>?credential=<cred>')` with `{type:'exec', id, command, args, cwd}` /
  `{type:'exec_result', ...}` message shapes, matching the legacy contract above exactly (including
  the 310s client-side timeout tuned to the legacy 300s server timeout).

None of this is MCP — it is raw HTTP/WS. Zero-frontend-change parity (a stated goal of Plan 028)
therefore requires an isolated Rust compatibility adapter reproducing section 1 exactly, deferred to
Phase 4 as scoped. This run does not implement it; only the MCP core (Phase 1/2) is in scope.

## 3. MCP `2026-07-28` contract frozen for this implementation

No live web access was available to re-verify the spec text in this session. The following is
carried over from the plan document's own citations/description (`Streamable HTTP`, stateless
core, JSON-RPC 2.0 base, routing headers) plus conservative, spec-shaped choices where the plan
text does not give an exact wire detail. **Assumptions are marked explicitly** — they should be
checked against the live spec before Phase 2 is considered conformance-final, not just
protocol-shaped.

- Transport: Streamable HTTP only. One route, `POST /mcp`, JSON body in, JSON body out (no SSE
  stream in this phase — streaming responses are not implemented; every `tools/call` response is
  a single JSON object, which is a spec-legal degenerate case of Streamable HTTP for
  non-streaming tools). **Assumption**: not implementing the optional `GET /mcp` SSE-upgrade path
  in this phase — no MCP client in this repo's scope needs it yet, and the plan defers it if a
  concrete client needs it later.
  Content type: `application/json` in, `application/json` out (no legacy HTTP+SSE
  `text/event-stream` used).
  - HTTP status codes: `200` for a well-formed JSON-RPC response (including a JSON-RPC-level
    error object — JSON-RPC errors are not necessarily HTTP errors), `400` for a transport-level
    parse/shape failure (invalid JSON, missing `jsonrpc`, wrong protocol version, oversized body),
    `404`/`405` for wrong path/method.
- Required header: `MCP-Protocol-Version: 2026-07-28`. **Decision for this implementation**: the
  header is *required* on every `POST /mcp` request (not merely validated-if-present as the
  pre-existing stub code did) — missing or mismatched version fails closed with `400` and a
  JSON-RPC `-32600 Invalid Request` envelope naming the expected version. This matches the plan's
  instruction to "validate `MCP-Protocol-Version` ... where applicable" and the security invariant
  of failing closed on ambiguous protocol state.
- JSON-RPC 2.0 envelope: `{jsonrpc:"2.0", id, method, params}` in;
  `{jsonrpc:"2.0", id, result}` or `{jsonrpc:"2.0", id, error:{code,message,data?}}` out. Standard
  error codes used: `-32700` parse error, `-32600` invalid request, `-32601` method not found,
  `-32602` invalid params, `-32603` internal error. Tool-execution-domain errors (e.g. "tool not
  yet implemented") are returned as `tools/call` *results* with `isError: true` per MCP tool-result
  convention, not as JSON-RPC protocol errors — a failing tool call is not a protocol failure.
- Methods implemented this run: `tools/list`, `tools/call`. `initialize`/`server/discover` are
  kept as a thin capability-announcement method (pre-existing in the stub) since a JSON-RPC/HTTP
  client still benefits from a self-description endpoint, but per the plan's explicit instruction
  this is **not** treated as a stateful session handshake — no `Mcp-Session-Id` is issued or
  required, and `tools/list`/`tools/call` work identically whether or not `initialize` was ever
  called. This is the stateless-core reading of the frozen spec.
- Body size limit enforced by `axum::extract::DefaultBodyLimit` before JSON deserialization (see
  resource limits below) so an oversized request is rejected before allocating a parsed value.

## 4. Canonical MCP tool catalog (1:1 with Plan 027 binaries)

| MCP tool name | Rust CLI binary | Notes |
| --- | --- | --- |
| `terminal_exec` | `terminal-tool` | args: `command` (string, required), `args` (string[], default `[]`), `cwd` (string, optional), `timeout_ms` (integer, optional, default 30000) |
| `http_fetch` | `curl-tool` | args: `url` (string, required), `method` (string, default `GET`), `headers` (object<string,string>, optional), `data` (string, optional), `timeout_ms` (integer, default 30000) |
| `web_search` | `searxng-search-tool` | args: `query` (string, required), `base_url` (string, optional, default `http://127.0.0.1:8888`) |

Names are snake_case MCP tool identifiers distinct from the binary filenames, since MCP tool names
are a client-facing API surface and the binary names are an internal implementation detail. Schemas
are implemented in `packages/rust-tools/src/relay_agent/mcp.rs::tool_catalog()` as JSON Schema
2020-12-compatible object schemas. Execution/dispatch is explicitly **out of scope this run**
(Phase 3) — `tools/call` validates the tool name and argument shape against the schema and then
returns a structured `isError: true` result with message `"tool execution not implemented (Phase 3)"`
rather than invoking a process.

## 5. Authorization model (decision, not implemented this run)

- Local MCP endpoint (`POST /mcp`): localhost-bind (`127.0.0.1` only) is the transport-level
  boundary for this phase. No MCP-level OAuth/bearer auth is implemented yet (Phase 5 scope) —
  documented here as a known gap, not silently assumed away.
  before deferring MCP-level auth) than reusing the legacy pairing credential for the MCP
  endpoint, since pairing is a browser/session concept and MCP is meant to be client-agnostic.
- Legacy compat endpoints keep their own pairing/session model unchanged (section 1), isolated
  from the MCP core per the plan's explicit requirement.

## 6. Frozen resource limits (concrete numbers + rationale)

| Limit | Value | Rationale |
| --- | --- | --- |
| MCP HTTP request body max | 1 MiB | Generous for JSON-RPC tool-call payloads (tool args are small strings/arrays); well below default OS socket buffers, prevents trivial memory-exhaustion from a single request. |
| Legacy WS compat message max | 1 MiB | Matches HTTP body limit for consistency; exec commands/args are short strings, stdout/stderr are separately bounded on the way out (Phase 4 concern). |
| MCP JSON-RPC message/frame max | 1 MiB | Same as HTTP body max — Streamable HTTP has no separate framing layer to bound in this phase. |
| Max single tool argument (string field) | 64 KiB | A single arg/URL/query well beyond this is almost certainly misuse, not a legitimate tool call; still comfortably fits typical shell command lines and URLs. |
| stdout/stderr capture limit per tool call | 2 MiB each | Matches Plan 027 CLI tool conventions (bounded, truncate-and-flag rather than unbounded buffering); large outputs should be paginated/streamed by the caller, not held in one JSON-RPC response. |
| Max concurrent tool executions (global) | 4 | Local single-user machine; bounds fork-bomb-via-repeated-tool-call risk without being so low it serializes normal chat-driven usage. |
| Max concurrent tool executions (per session, once sessions exist) | 2 | Leaves headroom for one interactive command plus one background one without one session starving the global pool. |
| Max tool execution duration | 300,000 ms (5 min) | Matches the legacy relay's `execTimeoutMs` default exactly — preserves compatibility expectation for long-running commands like `npm install`. |
| Pairing attempt rate (legacy compat, Phase 4) | 5 attempts / 60s per source | Pairing token is only 5-minute-lived and single-use already; a small attempt cap blocks brute-force guessing of the 16-byte token without needing full rate-limit infra. |
| Max tool-call queue depth | 16 | If the concurrency limit is hit, callers queue briefly rather than being rejected outright, but an unbounded queue is its own resource leak; 16 is 4x the global concurrency limit as a shock absorber, not a promise of throughput. |

These numbers are enforced starting in Phase 2 only where they gate the transport itself (HTTP body
limit); the rest (tool output, concurrency, queue depth, pairing rate) are Phase 3/4/5 concerns and
are recorded here so later phases implement the frozen numbers rather than picking new ones ad hoc.
