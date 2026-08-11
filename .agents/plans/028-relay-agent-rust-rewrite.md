# 028 — Relay agent: full Rust rewrite + MCP server

**Status: IN FLIGHT — implementation plan only.** No rewrite is complete until strict MCP/protocol parity, Nuxt E2E compatibility, security/resource-limit verification, standalone release verification, and complete removal of the Node.js relay runtime all pass.

## Context

Plan 027 migrated the three general-purpose CLI tools to Rust. The remaining executable runtime is `packages/relay-agent`, currently implemented in Node.js/TypeScript and packaged with `@yao-pkg/pkg`.

The relay agent is a **local MCP server / execution bridge**. It exists so the Nuxt application can use the local Rust CLI tools without browser-side process execution. The same MCP tool surface must be reusable by future MCP clients, including a future ChatGPT integration, so this rewrite must implement the **actual Model Context Protocol**, not a proprietary MCP-like wrapper.

The Rust relay must therefore become a small standalone native binary that:

1. exposes a standards-compliant MCP server;
2. preserves the existing Nuxt-facing local pairing/connection behavior where compatibility is required;
3. executes the Rust CLI tools through a controlled local execution layer;
4. keeps tool definitions and execution handlers independent from the transport so Nuxt, local MCP hosts, and future remote MCP deployments do not require separate implementations.

The current Node implementation and Nuxt consumers are the compatibility source of truth for legacy behavior. The MCP specification is the source of truth for new MCP protocol behavior.

## Goals

- Rewrite `packages/relay-agent` from Node.js/TypeScript to 100% Rust.
- Produce a standalone `relay-agent` native binary with no Node.js/V8/libnode runtime dependency.
- Implement **proper MCP server semantics**, not merely rename the existing WebSocket protocol.
- Target the current MCP specification (`2026-07-28`) and document any intentionally supported compatibility version. The current specification is stateless at the protocol core and uses Streamable HTTP; legacy HTTP+SSE is deprecated. citeturn0search0turn0search2
- Make the MCP tool catalog reusable across transports and clients.
- Preserve existing Nuxt local-terminal behavior with zero functional/source changes unless a Phase 0 contract audit proves a compatibility fix is unavoidable.
- Reuse the Rust CLI tools from Plan 027 rather than duplicating their implementations in the relay.
- Preserve localhost-only execution for the local agent and fail closed on browser-originated access.
- Provide a clean path to future ChatGPT/MCP-client integration without rewriting the tool layer.
- Remove Node.js, `@yao-pkg/pkg`, and relay-agent-specific JS build/runtime dependencies.
- Build and publish native Rust artifacts directly with Cargo.

## Important MCP deployment boundary

A local MCP server bound to `127.0.0.1` is directly usable only by a client/runtime that can reach that machine. A cloud-hosted ChatGPT integration cannot directly connect to a user's localhost process. Therefore:

- **Local Nuxt/browser use:** Streamable HTTP to `127.0.0.1:<port>` through the existing local relay flow.
- **Local MCP hosts:** support the standard local MCP transport selected by the client where practical; do not invent a proprietary transport.
- **Future ChatGPT/cloud use:** reuse the same MCP tool definitions/handlers behind a separately deployed, authenticated MCP endpoint when the product architecture requires remote access. Do not expose the local execution relay publicly just to make a cloud client work.
- The plan does **not** claim that a localhost binary alone makes ChatGPT able to execute commands on the user's machine.

This avoids doing the tool implementation twice while keeping the local security boundary intact.

## Scope boundary

- **In scope:** Rust relay runtime, MCP server, MCP tool catalog/handlers, local execution bridge, legacy Nuxt compatibility layer where required, local auth/pairing, CLI lifecycle, release pipeline, security/resource limits, tests, and removal of the Node implementation.
- **Out of scope:** migrating Nuxt/Vue/TypeScript to Rust; changing the web application runtime; exposing localhost execution remotely; implementing arbitrary OS sandboxing; replacing the Plan 027 CLI tools; building a second tool implementation specifically for ChatGPT.

## Architecture

```text
                         MCP clients
                 ┌──────────┴──────────┐
                 │                     │
          Nuxt local UI          Future MCP host
                 │                     │
                 │ Streamable HTTP     │ standard MCP transport
                 ▼                     ▼
          ┌──────────────────────────────────┐
          │        Rust relay-agent          │
          │                                  │
          │  MCP protocol / transport       │
          │  auth + localhost policy        │
          │  tool registry                  │
          │  tool execution dispatcher      │
          └───────────────┬──────────────────┘
                          │
                    local tool calls
                          ▼
          ┌──────────────────────────────────┐
          │        Rust CLI tools            │
          │ terminal-tool / curl-tool /      │
          │ searxng-search-tool              │
          └──────────────────────────────────┘
```

The relay is an **MCP server**, while the Plan 027 binaries remain the actual general-purpose CLI tools. The relay must not duplicate their core logic unless an MCP adapter genuinely needs a thin argument/response mapping.

Preferred Rust package layout:

```text
packages/rust-tools/
├── Cargo.toml
└── src/
    ├── bin/
    │   ├── curl-tool.rs
    │   ├── relay-agent.rs
    │   ├── searxng-search-tool.rs
    │   └── terminal-tool.rs
    └── relay_agent/
        ├── mod.rs
        ├── config.rs
        ├── error.rs
        ├── mcp.rs
        ├── transport.rs
        ├── auth.rs
        ├── pairing.rs
        ├── tools.rs
        ├── execution.rs
        ├── limits.rs
        ├── http_compat.rs
        ├── websocket_compat.rs
        └── pidfile.rs
```

The binary entrypoint remains thin. Protocol, tool registry, auth, execution, and lifecycle logic must be independently testable.

## MCP protocol requirements

### Protocol version

- [ ] Freeze the MCP specification version during Phase 0.
- [ ] Target **MCP `2026-07-28`** for the new server contract.
- [ ] Do not implement the removed legacy `initialize`/`initialized` + `Mcp-Session-Id` model as the primary protocol.
- [ ] Do not build new dependencies on deprecated legacy HTTP+SSE transport. Streamable HTTP is the required HTTP transport. citeturn0search0turn0search2
- [ ] If backward compatibility with an older MCP client is required, isolate it behind an explicit compatibility layer and test it separately.

### JSON-RPC / MCP methods

Implement the MCP methods/extensions actually required by the selected spec and product scope. At minimum the tool-server surface must correctly support:

- `server/discover` when required by the selected spec/client flow;
- `tools/list`;
- `tools/call`;
- protocol errors and JSON-RPC error semantics;
- capability advertisement appropriate to the implementation.

Do **not** implement deprecated/removed MCP methods merely because they existed in an older SDK. The current spec explicitly removed the old initialization/session handshake and redesigned long-lived server-to-client interactions. citeturn0search0

### Tool catalog

Expose the Plan 027 capabilities through MCP tools with stable names, descriptions, JSON Schema input definitions, and deterministic output/error semantics.

The tool registry must be transport-independent:

```text
MCP request
   ↓
validated tool name + schema
   ↓
tool registry
   ↓
execution adapter
   ↓
Rust CLI tool
   ↓
normalized MCP result/error
```

Tool schemas must be explicit JSON Schema 2020-12-compatible definitions where required by the selected MCP specification/client. Do not accept arbitrary unvalidated JSON and pass it to a process.

Tool annotations/metadata must accurately describe risk and behavior. Treat annotations as descriptive metadata, not as an authorization boundary.

### `tools/call` execution

- [ ] Validate tool name.
- [ ] Validate arguments against the declared schema.
- [ ] Apply authorization before execution.
- [ ] Apply resource/concurrency limits.
- [ ] Execute only the intended Rust CLI tool.
- [ ] Normalize stdout/stderr/exit status into the MCP result contract.
- [ ] Return structured tool errors rather than leaking process internals.
- [ ] Preserve cancellation/timeout semantics where supported by the selected MCP version.
- [ ] Never let tool arguments become an implicit shell command.

### Streamable HTTP

For the HTTP MCP endpoint:

- [ ] Implement the transport requirements of MCP `2026-07-28`.
- [ ] Validate `MCP-Protocol-Version` and the required MCP routing headers where applicable.
- [ ] Preserve JSON-RPC request/response semantics.
- [ ] Support the response/content types required by the selected transport mode.
- [ ] Enforce request/message size limits before unbounded allocation.
- [ ] Do not rely on a hidden server-side session as an authorization boundary.
- [ ] Implement required CORS behavior for Nuxt without wildcarding security-sensitive origins.
- [ ] Add interoperability tests using an official/standards-compliant MCP client or protocol harness where available.

The current 2026-07-28 spec adds method/name HTTP headers for routing and a stateless core; these must be treated as protocol requirements rather than custom headers invented by this project. citeturn0search0

### Authorization

The local relay has a separate browser-local authentication problem from generic MCP authorization. Keep them layered:

```text
HTTP transport
  ↓
localhost + Origin/Host policy
  ↓
MCP authorization (when required)
  ↓
tool authorization/policy
  ↓
tool execution
```

For any remotely deployed MCP endpoint in the future, use standards-based OAuth/protected-resource authorization rather than reusing the localhost pairing credential. Current MCP authorization guidance requires proper authorization-server/resource discovery and HTTP `401` behavior for protected resources. citeturn0search3

For the local Nuxt relay:

- [ ] Pairing credentials are single-use and short-lived.
- [ ] Session credentials are random, expiring, revocable, and race-safe.
- [ ] Credentials never appear in logs or error messages.
- [ ] Missing/wrong Origin fails closed.
- [ ] Missing/wrong Host fails closed.
- [ ] No debug/test authentication bypass exists.

## Legacy Nuxt compatibility contract

The existing `/health`, `/pair`, `/revoke`, and WebSocket execution protocol are **legacy compatibility surfaces**, not the MCP protocol itself. Phase 0 must determine whether Nuxt can be moved to MCP directly without source changes. If not, retain a thin, isolated compatibility adapter in Rust while the MCP endpoint remains standards-compliant.

Required audit:

- [ ] `GET /health` exact status/body/content type.
- [ ] `POST /pair` exact request/response/error contract.
- [ ] `POST /revoke` exact request/response/error contract.
- [ ] `OPTIONS`/CORS behavior.
- [ ] WebSocket path/query/header/close behavior.
- [ ] `exec`/`exec_result` message semantics.
- [ ] CLI defaults and environment precedence.
- [ ] Existing frontend tests and download/update flow.

Do not describe the legacy WebSocket API as MCP. It is a compatibility adapter until/unless the Nuxt client is migrated to the MCP transport in a separate, explicitly approved scope.

## Security invariants

1. Local mode binds only to `127.0.0.1`.
2. Browser-facing compatibility endpoints require exact configured Origin and valid Host; missing values fail closed.
3. MCP HTTP authorization must follow the selected MCP specification; localhost pairing is not reused as a remote OAuth credential.
4. Pairing is single-use, short-lived, cryptographically random, and atomically consumed.
5. Session credentials are random, expiry-bound, revocable, race-safe, and never logged.
6. No wildcard Origin or hidden/debug/test bypass.
7. Request bodies, MCP messages, tool arguments, command output, and concurrent executions are bounded.
8. Tool names and arguments are validated before process execution.
9. No shell interpolation is introduced by the relay.
10. Timeouts terminate the intended process tree and reap children.
11. Pidfile acquisition/release is atomic and ownership-safe.
12. Error responses do not expose secrets, environment variables, stack traces, or raw credentials.
13. Remote deployment, if added later, must use standards-based MCP authorization and must not expose the local unauthenticated execution path.

## Resource limits

Freeze concrete values during Phase 0. At minimum define and test:

- HTTP/MCP body limit;
- WebSocket compatibility message limit;
- MCP message/frame limit;
- maximum tool argument payload;
- stdout/stderr limit;
- per-session and global concurrent tool executions;
- maximum execution duration;
- pairing attempt rate/limit;
- maximum tool-call queue depth if requests can queue.

Limits must fail deterministically. Never silently truncate command input or tool arguments.

## CLI contract

Use `clap` and preserve the existing public flags:

- `--dir`, `-d`: default working directory, falling back to the OS home directory;
- `--port`, `-p`: default `47821`;
- `--origin`, `-o`: allowed Nuxt origin, with `RELAY_AGENT_ORIGIN` as environment fallback;
- `stop --port <port>`: stop the port-scoped local agent.

Validate configuration before binding. Do not normalize Origin in a way that broadens trust.

## Command execution

Use `tokio::process::Command` and keep the execution adapter explicit.

- [ ] Map MCP tool arguments to the existing Plan 027 Rust CLI binaries.
- [ ] Never construct a shell command from untrusted MCP/HTTP input.
- [ ] Preserve stdout, stderr, exit code, timeout, and error semantics.
- [ ] Bound output and argument sizes.
- [ ] Enforce concurrency limits.
- [ ] Kill the intended process group/tree on timeout where supported.
- [ ] Do not block Tokio workers during child cleanup.
- [ ] Test spaces, quotes, shell metacharacters, Unicode, empty args, cwd, non-zero exit, missing binary, timeout, and output overflow.

## PID / lifecycle

- [ ] Atomic exclusive pidfile/lock.
- [ ] Stale pidfile recovery.
- [ ] Second-instance rejection.
- [ ] Safe `stop --port` on supported OSes.
- [ ] Clean SIGINT/SIGTERM shutdown.
- [ ] Delete only the pidfile owned by the current process.
- [ ] Protect against symlink/path races.
- [ ] Repeated start/stop integration tests.

## Binary and supply chain

- [ ] Reuse `packages/rust-tools` toolchain/MSRV policy from Plan 027.
- [ ] Keep dependency/features minimal.
- [ ] Benchmark `opt-level = z` vs `s`/`3`; choose from measured evidence.
- [ ] Measure LTO/codegen-unit/strip tradeoffs.
- [ ] Verify no Node/V8/libnode dependency.
- [ ] Review `Cargo.lock` changes.
- [ ] Run project-approved `cargo audit`/`cargo deny` equivalent.
- [ ] Record release binary size, startup time, and basic resource usage.
- [ ] Produce an artifact manifest with target, commit/version, SHA-256, and size.

## Implementation phases

### Phase 0 — Contract + MCP freeze — [x] DONE

- [x] Reverse-engineer the current Node relay and all Nuxt consumers.
- [x] Freeze legacy HTTP/WS compatibility contract. See `.agents/plans/028-phase0-contract-audit.md` section 1.
- [x] Freeze MCP `2026-07-28` contract and supported transports. See audit doc section 3 — includes explicit assumptions since live spec re-verification was not possible this session.
- [x] Decide whether Nuxt will consume MCP directly in this plan or retain the isolated legacy adapter for zero frontend changes. Decision: retain the isolated legacy adapter (Phase 4) — `app/composables/useRelayAgent.ts` uses the raw `/pair`/`/revoke`/WS contract directly, not MCP. See audit doc section 2.
- [x] Define the canonical MCP tool names, descriptions, schemas, annotations, and output/error contract. See audit doc section 4 and `packages/rust-tools/src/relay_agent/mcp.rs::tool_catalog()`.
- [x] Map each MCP tool to exactly one Plan 027 Rust CLI capability. `terminal_exec`→`terminal-tool`, `http_fetch`→`curl-tool`, `web_search`→`searxng-search-tool`.
- [x] Define authorization model for local vs future remote deployment. See audit doc section 5 (MCP-level auth itself is explicitly deferred to Phase 5, documented as a known gap, not implemented).
- [x] Freeze concrete resource limits. See audit doc section 6.
- [x] Create compatibility and MCP conformance fixtures before deleting Node code. MCP conformance fixtures: `packages/rust-tools/tests/mcp_transport_tests.rs` (16 tests). No Node code was touched or deleted this run.

### Phase 1 — Rust foundation — [x] DONE

- [x] Add `relay-agent` `[[bin]]` under `packages/rust-tools`. Pre-existing in `Cargo.toml`, confirmed working.
- [x] Add minimal Axum/Tokio/Clap and MCP protocol dependencies or implement the small protocol layer directly when this reduces dependency surface without sacrificing interoperability. Implemented directly on `axum`/`tokio`/`clap`/`serde_json`, no MCP SDK crate added.
- [x] Keep transport, protocol, tool registry, auth, execution, and lifecycle modules separate. This run populates `config.rs`, `error.rs`, `mcp.rs`, `transport.rs` only. Per the task scope, `auth.rs`/`pairing.rs`/`tools.rs`/`execution.rs`/`limits.rs`/`http_compat.rs`/`websocket_compat.rs`/`pidfile.rs` were deliberately **not** created as empty scaffolding — they are still TODO for Phase 3/4/5, tracked there rather than as unpopulated files.
- [x] Keep formatting/Clippy clean. `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` both pass with zero warnings.

### Phase 2 — MCP server — [x] DONE (execution itself intentionally not implemented — Phase 3)

- [x] Implement MCP `2026-07-28` protocol core. `packages/rust-tools/src/relay_agent/mcp.rs`.
- [x] Implement Streamable HTTP. `POST /mcp` JSON-in/JSON-out; no SSE upgrade path implemented yet (see audit doc section 3 assumption — no client in scope needs it).
- [x] Implement `server/discover`/capability discovery as required by the frozen spec/client matrix. Implemented as a stateless capability-announcement method (no session handshake), per the audit doc's stateless-core reading.
- [x] Implement `tools/list`. Returns the full 3-tool catalog with JSON Schema 2020-12-compatible `inputSchema`.
- [x] Implement `tools/call`. Validates tool name + params shape and dispatches to a structured `isError:true` "not implemented" result (Phase 3 owns real execution) — never a panic or 500.
- [x] Implement JSON-RPC errors and protocol-version validation. `error.rs::McpError` + reserved JSON-RPC codes; `MCP-Protocol-Version` header is required (not merely validated-if-present) and fails closed.
- [x] Implement required routing headers and content types. `MCP-Protocol-Version` header enforced; `Content-Type: application/json` enforced; request body bounded via `DefaultBodyLimit` (1 MiB, frozen in audit doc) before parsing.
- [ ] Add official/client interoperability tests. Not done — no official/standards-compliant MCP client or protocol harness was available in this session; only this project's own integration tests (`mcp_transport_tests.rs`) were added. Left unchecked deliberately.

### Phase 3 — Tool registry and execution — [ ] TODO

- [ ] Register Plan 027 Rust CLI tools.
- [ ] Validate JSON Schema arguments.
- [ ] Dispatch without shell interpolation.
- [ ] Apply auth, limits, timeout, and concurrency policy.
- [ ] Normalize results/errors.
- [ ] Add unit/integration tests per tool.

### Phase 4 — Legacy Nuxt compatibility — [ ] TODO

- [ ] Implement `/health`, `/pair`, `/revoke`, CORS, and existing WebSocket adapter only if Phase 0 proves the frontend still requires them.
- [ ] Preserve exact behavior.
- [ ] Add regression tests.
- [ ] Keep compatibility code isolated from the MCP core.

### Phase 5 — Lifecycle/security hardening — [ ] TODO

- [ ] Pairing/session state machine.
- [ ] Origin/Host validation.
- [ ] Resource limits.
- [ ] Timeout/process-tree cleanup.
- [ ] Pidfile/stop lifecycle.
- [ ] Security regression suite.

### Phase 6 — Real Nuxt E2E — [ ] TODO

- [ ] Start the real Rust binary.
- [ ] Use the existing Nuxt UI with zero source changes unless explicitly approved.
- [ ] Pair/connect.
- [ ] Execute a terminal command.
- [ ] Verify stdout/stderr/exit status/error/timeout rendering.
- [ ] Verify revoke/disconnect.
- [ ] Verify browser-origin security.
- [ ] Run against the real binary, not a mock server.

### Phase 7 — Remove Node runtime — [ ] TODO

Only after MCP + Nuxt parity is proven:

- [ ] Delete `packages/relay-agent/src/*`.
- [ ] Delete Node CLI/pidfile/build scripts.
- [ ] Remove relay-agent-only Node dependencies.
- [ ] Remove `@yao-pkg/pkg` repository-wide.
- [ ] Remove obsolete package/build references.
- [ ] Prove no relay-agent executable JS/TS remains.

### Phase 8 — Native release — [ ] TODO

- [ ] Rewrite `.github/workflows/release-relay-agent.yml` to use Cargo directly.
- [ ] Build all supported targets with appropriate runners/cross-compilation.
- [ ] Publish stable artifact names expected by existing consumers.
- [ ] Generate and verify checksums.
- [ ] Verify Node-free standalone execution.
- [ ] Test published assets from a clean environment/tag.

### Phase 9 — Production hardening — [ ] TODO

- [ ] Dependency/license/security policy checks.
- [ ] Size/startup/resource measurements.
- [ ] Release artifact manifest.
- [ ] Provenance/signing where repository policy supports it.
- [ ] Full MCP interoperability matrix.
- [ ] Full Nuxt E2E matrix.

### Phase 10 — Closeout — [ ] TODO

- [ ] Full Rust tests green.
- [ ] MCP conformance/interoperability tests green.
- [ ] Security/resource-limit tests green.
- [ ] Nuxt E2E green with no frontend source change.
- [ ] No Node relay source/build/runtime remains.
- [ ] No `@yao-pkg/pkg` remains.
- [ ] All release artifacts are standalone and verified.
- [ ] Published binaries are smoke-tested.
- [ ] Documentation describes MCP as MCP, not as a proprietary WebSocket protocol.
- [ ] Plan is marked `COMPLETED` only after final CI/release evidence is recorded.

## Test strategy

### MCP conformance

- [ ] Protocol version handling.
- [ ] Capability discovery.
- [ ] `tools/list` schema validity and deterministic ordering.
- [ ] `tools/call` success/error semantics.
- [ ] JSON-RPC malformed request/error cases.
- [ ] Streamable HTTP content types and headers.
- [ ] Required MCP routing headers.
- [ ] Cancellation/timeout semantics where supported.
- [ ] Interoperability with at least one standards-compliant MCP client/harness.

### Security

```text
missing Origin              → reject
wrong Origin                → reject
wildcard Origin             → reject
missing/wrong Host          → reject
invalid credential           → reject
reused pairing token         → reject
racing pairing requests      → one success only
invalid MCP authorization    → reject
oversized request/message    → bounded rejection
oversized tool output        → bounded rejection
tool not in registry         → reject
invalid tool arguments       → reject
execution timeout            → process cleanup
```

No security test may use a hidden production bypass or relaxed debug configuration.

### Nuxt E2E

```text
Nuxt UI
  ↓
local Rust relay
  ↓
MCP/compatibility transport
  ↓
tool registry
  ↓
Plan 027 Rust CLI
  ↓
result
  ↓
Nuxt UI
```

The final gate must exercise the real Rust binary and the real frontend flow. Mock-only tests are insufficient.

## CI gates

- [ ] `cargo fmt --check`.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] `cargo test --workspace`.
- [ ] MCP conformance/interoperability tests.
- [ ] Security/resource-limit tests.
- [ ] Real Nuxt E2E tests.
- [ ] Repository-wide `@yao-pkg/pkg` absence check.
- [ ] Repository-wide relay-agent JS/TS executable absence check.
- [ ] Release build for every supported target.
- [ ] Published artifact smoke test.
- [ ] Artifact naming/checksum/manifest verification.
- [ ] Node-free runtime verification.
- [ ] Dependency/license/security policy check.
- [ ] No unrelated Nuxt regressions.

## Definition of Done

Plan 028 is **CLOSED** only when:

- [ ] `relay-agent` is entirely Rust and the binary is the sole runtime entrypoint.
- [ ] It is a proper MCP server targeting the frozen MCP specification, not an MCP-like custom protocol.
- [ ] The MCP tool catalog maps cleanly to the Plan 027 Rust CLI tools.
- [ ] Nuxt works with zero functional/source changes, or any exception is explicitly approved and documented.
- [ ] Legacy compatibility, if retained, is isolated from the MCP core.
- [ ] MCP protocol and transport conformance tests pass.
- [ ] Origin/Host/auth security is fail-closed.
- [ ] Pairing/session lifecycle is single-use, race-safe, expiry-bound, and revocable.
- [ ] Resource limits and process cleanup are enforced and tested.
- [ ] Node.js/TypeScript relay source/build/runtime is removed.
- [ ] `@yao-pkg/pkg` is removed from the monorepo.
- [ ] Release CI builds native binaries directly with Cargo.
- [ ] Published binaries are standalone, checksummed, and smoke-tested.
- [ ] Full CI is green and final release evidence is recorded.

## Rollback

Keep the Node/pkg release artifact available until the Rust MCP server, Nuxt compatibility path, and published native artifacts have all passed final verification. If a Rust release fails, restore the known-good previous release, keep Plan 028 `IN FLIGHT`, fix the implementation, and repeat the full conformance/E2E/release gate.

## Evidence log

Record final evidence as implementation progresses:

- Contract inventory: `[x]` `.agents/plans/028-phase0-contract-audit.md` section 1 (legacy HTTP/WS) + section 4 (MCP tool catalog).
- MCP specification/conformance matrix: `[x]` `.agents/plans/028-phase0-contract-audit.md` section 3 (frozen contract, assumptions noted); tests in `packages/rust-tools/tests/mcp_transport_tests.rs` (16/16 passing) cover protocol-version handling, `tools/list` schema shape, `tools/call` structured-error semantics, malformed JSON-RPC, oversized body, and missing/invalid `MCP-Protocol-Version` header. No official MCP client/harness interoperability run yet.
- Threat model/resource limits: `[x]` `.agents/plans/028-phase0-contract-audit.md` section 6 (frozen numbers); only the HTTP body limit is enforced in code so far (Phase 2 scope), the rest are Phase 3/4/5 scope.
- Rust implementation: partial `[~]` — Phase 1/2 only (config/error/mcp/transport modules + relay-agent binary entrypoint). Tool registry, execution, auth, pairing, limits enforcement, legacy compat, pidfile lifecycle remain TODO (Phase 3+).
- MCP interoperability tests: `[ ]`
- Security regression suite: `[ ]`
- Nuxt E2E parity: `[ ]`
- Node source/runtime removal: `[ ]`
- `@yao-pkg/pkg` removal: `[ ]`
- Release workflow migration: `[ ]`
- Dependency/security policy checks: `[ ]`
- Published artifact smoke tests: `[ ]`
- Artifact manifest/checksums: `[ ]`
- Final CI run: `[ ]`
- Final release/tag: `[ ]`
