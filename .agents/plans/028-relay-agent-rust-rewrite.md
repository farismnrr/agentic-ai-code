# 028 — Relay agent: full Rust rewrite

**Status: IN FLIGHT — implementation plan only; no rewrite work is considered complete until strict frontend/API parity, security/resource-limit verification, standalone release verification, and removal of the Node.js implementation all pass.**

## Context

Plan 027 migrated the three general-purpose CLI tools to Rust. The remaining executable CLI/runtime dependency is `packages/relay-agent`, which is still implemented as Node.js/TypeScript and packaged with `@yao-pkg/pkg`.

The current relay agent is a localhost-only HTTP/WebSocket bridge used by the Nuxt application for local terminal execution. The rewrite must preserve the browser-facing contract exactly while replacing the runtime and packaging layer with a small standalone Rust binary.

Current implementation to reverse-engineer before changing:

- `packages/relay-agent/src/server.ts` — HTTP/WebSocket server, pairing, credential storage, command execution.
- `packages/relay-agent/src/index.ts` — server export.
- `packages/relay-agent/bin/cli.mjs` — CLI parsing, lifecycle, singleton/pidfile handling.
- `packages/relay-agent/bin/pidfile.mjs` — atomic pidfile/lock and stale-lock recovery.
- `packages/relay-agent/package.json` — Node build/package contract and `@yao-pkg/pkg` dependency.
- `.github/workflows/release-relay-agent.yml` — current Node/pkg release pipeline.
- Nuxt local-terminal composables/components/API integration — the consumer contract that must not require frontend changes.

The final architecture is:

```text
Nuxt browser
    │
    │ HTTP / WebSocket
    │ Origin = configured Nuxt origin
    ▼
127.0.0.1:<port>
    │
    ▼
standalone relay-agent Rust binary
    │
    ├── /health
    ├── /pair
    ├── /revoke
    └── WebSocket ?credential=...
             │
             ▼
      tokio::process::Command
             │
             ▼
       local OS process
```

The hosted server remains outside the terminal data path. This plan changes the relay-agent runtime only; **Nuxt remains Nuxt/TypeScript** and is not being migrated to Rust.

## Goals

- Rewrite `packages/relay-agent` from Node.js/TypeScript to 100% Rust.
- Produce a single native executable named `relay-agent` for supported release targets.
- Remove the requirement for Node.js, V8, `node_modules`, `esbuild`, and `@yao-pkg/pkg` to run or package the relay agent.
- Preserve the browser/API/WebSocket contract exactly enough that the Nuxt frontend requires **zero functional changes**.
- Preserve localhost-only binding and fail-closed Origin/Host validation.
- Preserve one-time pairing, credential revocation, command execution, stdout/stderr/exit-code reporting, timeout behavior, and lifecycle semantics.
- Enforce explicit resource limits so a localhost browser request cannot create an unbounded process, memory, output, or WebSocket workload.
- Keep the final binary small by using a minimal dependency/features set and release-profile optimization.
- Make the release workflow build native Rust artifacts directly with Cargo.

## Non-goals / explicit scope boundary

- Do **not** migrate Nuxt, Vue, TypeScript, or the web application runtime to Rust.
- Do **not** redesign the relay-agent HTTP/JSON/WebSocket protocol.
- Do **not** add remote/non-localhost relay access.
- Do **not** add multi-user sharing, file transfer, or OS-level sandboxing.
- Do **not** turn `--dir` into a filesystem security boundary; it remains the default starting working directory, matching Plan 026's final behavior.
- Do **not** introduce a new server-side terminal path.
- Do **not** preserve Node.js as a fallback runtime after cutover.
- Do **not** add arbitrary command allowlists, filesystem sandboxes, or privilege-dropping behavior unless required by an independently approved security design; these would change the existing local-agent contract.

## Threat model and security invariants

The primary security boundary is **browser-originated access to a localhost process-execution service**. Binding to loopback reduces remote exposure but does not make the service trusted: any local browser page may attempt requests, and WebSocket credentials must not substitute for browser-origin validation.

Required invariants:

1. Listener binds only to `127.0.0.1`; never `0.0.0.0`, `::`, or another externally reachable interface.
2. HTTP and WebSocket requests require a valid `Host` and exact configured `Origin`; missing values fail closed.
3. WebSocket credentials are never accepted without passing Origin/Host validation.
4. Pairing tokens are single-use, short-lived, cryptographically random, and never logged.
5. Session credentials are cryptographically random, expire according to the frozen contract, are revocable, and are never logged or returned except through the intended `/pair` response.
6. Credentials in URLs must never appear in application logs, error messages, telemetry, panic output, or test snapshots.
7. No wildcard Origin, permissive fallback, implicit localhost Origin, or debug/test-only authentication bypass is allowed.
8. Request bodies, WebSocket messages, command arguments, output buffers, and concurrent executions are bounded.
9. Timeouts terminate the intended process tree and leave no known orphan descendants.
10. Shutdown and pidfile cleanup are ownership-safe and race-safe.
11. Secrets and command output are not emitted through structured logs by default.
12. Error responses do not disclose credentials, filesystem secrets, environment variables, or internal stack traces.

### Resource limits

Freeze concrete limits during Phase 0 from the current implementation where they exist; otherwise choose conservative documented defaults and make them configurable only if the existing UX requires it. At minimum define and test:

- maximum HTTP request body size;
- maximum WebSocket frame/message size;
- maximum command string/argument payload size;
- maximum buffered stdout size;
- maximum buffered stderr size;
- maximum concurrent command executions per session and globally;
- maximum command execution duration;
- maximum pairing attempts per process/window if required by the existing flow.

When a limit is exceeded, fail deterministically with the existing or explicitly documented error contract; never silently truncate security-sensitive input.

## Architecture decisions

### 1. Workspace placement

Use the existing `packages/rust-tools/` Cargo package/workspace as the home for the new binary unless implementation evidence shows a separate Cargo workspace is materially cleaner. Preferred outcome:

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
        ├── http.rs
        ├── pairing.rs
        ├── websocket.rs
        ├── execution.rs
        ├── limits.rs
        └── pidfile.rs
```

Keep shared relay-agent logic out of the binary entrypoint. The `[[bin]]` should remain a thin CLI/bootstrap layer.

### 2. HTTP/WebSocket framework

Use `axum` on top of Tokio unless the contract audit demonstrates a concrete reason to use another mature Rust HTTP/WebSocket framework.

Required features should be kept minimal: HTTP routing, JSON, WebSocket upgrade, and Tokio runtime support. Avoid pulling in unrelated framework features.

### 3. CLI

Use `clap` derive parsing, matching the existing public flags:

- `--dir`, `-d`
- `--port`, `-p`
- `--origin`, `-o`
- `stop --port <port>`

`RELAY_AGENT_ORIGIN` remains an environment fallback for `--origin`.

`--dir` defaults to the OS user's home directory. `--port` defaults to `47821`.

Configuration validation must reject invalid ports, empty/invalid origins, unusable directories, and contradictory options before binding/listening. Do not silently normalize an Origin in a way that weakens exact-match security.

### 4. Pairing and credentials

Use a cryptographically secure OS-backed random source for pairing/session credentials. Store only the minimum in-memory state required by the current contract.

- Pairing token is short-lived and single-use.
- Successful `/pair` invalidates the pairing token atomically.
- Session credential is required for WebSocket upgrade.
- `/revoke` removes the credential from the active session set atomically.
- Invalid, expired, reused, or missing credentials fail closed.
- Credential state is concurrency-safe and cannot be replayed by racing `/pair` requests.
- Do not persist credentials to disk unless the existing frontend contract proves persistence is required.
- Credential entropy and expiry are explicitly documented and tested.

### 5. Localhost, Host, Origin, and HTTP security

The listener must bind only to `127.0.0.1`.

For HTTP requests and WebSocket upgrade:

- Require `Host` to match the supported localhost forms for the selected port, preserving current behavior.
- Require `Origin` to be present.
- Require `Origin` to equal the configured `--origin` / `RELAY_AGENT_ORIGIN` exactly; do not parse/normalize it into a broader allowlist unless the frozen Node contract requires that exact behavior.
- Reject missing/mismatched Origin with no fallback or wildcard behavior.
- Apply the same validation before WebSocket upgrade.
- Never trust the WebSocket credential as a substitute for Origin/Host validation.
- Return explicit `Content-Type: application/json` for JSON endpoints where the Node contract does so.
- Reject unsupported HTTP methods with the same status/shape required by the frozen contract.
- Apply request-body limits before deserialization where practical.

CORS behavior and preflight responses must remain compatible with the existing frontend. CORS must not become a permissive wildcard merely because the server is localhost-only.

### 6. Command execution

Use `tokio::process::Command` with shell execution disabled by default.

Preserve the current wire contract for the `exec` payload, including:

- `type: "exec"`
- `id`
- `command`
- `args`
- optional `cwd`

Preserve current command parsing semantics after capturing the exact frontend payload contract. In particular, do not silently change how a command string and explicit `args` are combined.

Execution response must preserve:

- `type: "exec_result"`
- `id`
- `success`
- `exitCode`
- `stdout`
- `stderr`
- `error` where applicable

Do not change field names, omission/null behavior, or success/failure semantics without explicit parity evidence.

Output collection must be bounded. If the current contract requires full output, use a documented maximum and return a deterministic overflow error rather than unbounded memory growth.

### 7. Timeout and process lifecycle

Commands must have the same effective default timeout as the current implementation and must terminate when the timeout expires.

Prefer a process-group-aware termination strategy on Unix so descendants cannot outlive a timed-out command. On Windows, use the platform-appropriate process-tree termination strategy if supported by the chosen implementation.

The implementation must not block the Tokio runtime while waiting for child-process cleanup. Tests must prove that timeout cleanup reaps the direct child and addresses descendants within the supported platform semantics.

### 8. PID file / singleton lifecycle

Preserve the current port-scoped singleton behavior:

- Acquire an atomic exclusive lock before starting the server.
- Detect and recover stale pidfiles.
- Reject a second live instance on the same port.
- `stop --port <port>` reads the port-scoped pidfile and sends the appropriate termination mechanism for the OS.
- Normal shutdown removes only the pidfile owned by the current process.
- SIGINT/SIGTERM must close the HTTP/WebSocket server and release state cleanly.
- Pidfile contents must be minimal and non-secret.
- Symlink/path races must not allow deletion of an unrelated file.

Use an OS-appropriate per-user runtime/state directory. Do not regress to an unsafe shared pidfile location merely to match the old Node implementation; preserve the behavior users rely on while retaining the current security/race fixes from Plan 026.

### 9. Small standalone binary

Use Cargo release builds with size-oriented settings where they do not compromise reliability:

- Benchmark `opt-level = "z"` versus `"s"`/`3` and keep the measured best result for the supported workload.
- `lto = "thin"` or `true` based on measured size/build-time tradeoff.
- `codegen-units = 1` where the size/build-time tradeoff is acceptable.
- `strip = "symbols"`/`true` for release artifacts where supported.
- Keep `panic = "abort"` optional; use it only after verifying panic/error handling and diagnostics remain acceptable.

Cargo officially supports size-oriented `opt-level` settings, LTO, codegen units, and stripping; the plan requires measurement rather than assuming one combination is universally smallest. citeturn0search0turn0search2

Measure the resulting binaries instead of claiming a size target without evidence. Record release artifact sizes and compare against the old pkg bundles.

## Strict API / protocol contract

Before implementation, create a contract inventory from the current Rust-free implementation and the Nuxt consumers. Capture exact:

- HTTP method/path/status/content-type/body shape/header behavior.
- CORS/preflight behavior.
- WebSocket path, query encoding, upgrade rejection behavior, close codes/reasons where observable.
- Pairing/session credential lifecycle.
- Command payload parsing and result serialization.
- Timeout/error semantics.
- CLI defaults and environment-variable precedence.
- Artifact names/download URLs.

### HTTP

#### `GET /health`

Must return HTTP 200 and preserve the existing JSON shape, including:

```json
{
  "status": "ok",
  "agent": "relay-agent",
  "defaultCwd": "..."
}
```

The exact serialization, field casing, content type, and headers must be verified against the frontend/tests before implementation is marked complete.

#### `POST /pair`

Input currently contains the pairing token. Preserve:

- success status and JSON field name for the generated session credential;
- invalid-token behavior;
- expired-token behavior;
- malformed JSON behavior;
- one-time token semantics;
- content type and error response shape.

Expected successful response currently uses `sessionCredential`.

#### `POST /revoke`

Preserve credential input, success response, invalid-credential behavior, content type/error shape, and the fact that a revoked credential can no longer open a WebSocket session.

#### `OPTIONS`

Preserve the frontend-required CORS/preflight behavior for the three HTTP endpoints, including allowed origin, methods, and headers. Test preflight independently.

### WebSocket

Connection URL remains equivalent to:

```text
ws://127.0.0.1:<port>/?credential=<sessionCredential>
```

The exact path must be confirmed from the Nuxt consumer before implementation freezes the route.

Upgrade must fail closed for:

- missing Origin;
- wrong Origin;
- missing/wrong Host;
- missing credential;
- invalid credential;
- revoked credential.

Credential query parsing must correctly handle URL encoding and must never log the raw URL.

### Messages

Incoming execution message:

```json
{
  "type": "exec",
  "id": "...",
  "command": "...",
  "args": [],
  "cwd": "..."
}
```

Outgoing execution result:

```json
{
  "type": "exec_result",
  "id": "...",
  "success": true,
  "exitCode": 0,
  "stdout": "...",
  "stderr": "..."
}
```

The examples are a starting point only; **the implementation must derive the final contract from the current Nuxt consumer and existing tests before changing the protocol**.

Unknown message types and malformed JSON must preserve the current error envelope and connection behavior. Oversized messages must be rejected deterministically before unbounded deserialization/allocation.

## Implementation phases

### Phase 0 — Contract freeze, threat model, and inventory — [ ] TODO

- [ ] Read the entire current relay-agent implementation, including CLI and pidfile modules.
- [ ] Identify every Nuxt caller/consumer of `/health`, `/pair`, `/revoke`, WebSocket URL, headers, query parameters, and message types.
- [ ] Identify all release/download consumers and expected artifact names.
- [ ] Record exact HTTP status codes, JSON shapes, content types, headers, CORS/preflight behavior, WebSocket path, close/error behavior, timeout values, default values, and environment-variable precedence.
- [ ] Record current process lifecycle behavior and the Phase 9 atomic pidfile semantics from Plan 026.
- [ ] Produce a threat model for browser→localhost access, credential theft/replay, origin bypass, resource exhaustion, command execution, and lifecycle races.
- [ ] Freeze concrete resource limits and document the rationale.
- [ ] Turn the inventory into contract tests/fixtures before deleting the Node implementation.
- [ ] Freeze a compatibility matrix mapping every old behavior to a Rust test.

### Phase 1 — Rust crate and binary skeleton — [ ] TODO

- [ ] Add `relay-agent` as a `[[bin]]` in `packages/rust-tools/Cargo.toml`.
- [ ] Add only required dependencies/features and justify every runtime dependency.
- [ ] Pin/verify compatible Rust toolchain/MSRV according to Plan 027 policy.
- [ ] Implement `clap` CLI and environment fallback.
- [ ] Implement configuration validation and default directory/port/origin handling.
- [ ] Add Rust module boundaries for HTTP, WebSocket, pairing, execution, lifecycle, limits, and errors.
- [ ] `cargo fmt --check` and Clippy must be clean from the first complete skeleton.

### Phase 2 — HTTP API parity — [ ] TODO

- [ ] Implement `GET /health`.
- [ ] Implement `POST /pair`.
- [ ] Implement `POST /revoke`.
- [ ] Implement OPTIONS/CORS behavior required by Nuxt.
- [ ] Implement exact Host/Origin validation.
- [ ] Implement method/content-type/body-size validation required by the frozen contract.
- [ ] Add unit/integration tests for success, malformed input, invalid credentials, expiry, reuse, wrong/missing Origin/Host, unsupported methods, and oversized requests.
- [ ] Compare Rust responses against the frozen contract fixtures byte-for-byte where practical and semantically where nondeterministic values exist.

### Phase 3 — WebSocket/auth parity — [ ] TODO

- [ ] Implement WebSocket upgrade on the exact existing path.
- [ ] Validate Origin/Host before upgrade.
- [ ] Validate session credential before upgrade.
- [ ] Preserve credential revocation semantics.
- [ ] Implement malformed/unknown message handling.
- [ ] Enforce WebSocket message/frame size limits.
- [ ] Enforce concurrent-execution limits.
- [ ] Add integration tests for accepted and rejected upgrade cases, including missing/wrong Origin, Host, credential, and URL-encoded credential.
- [ ] Add tests proving credentials never appear in logs or error output.

### Phase 4 — Command execution parity — [ ] TODO

- [ ] Implement `tokio::process::Command` execution.
- [ ] Preserve command/args parsing semantics.
- [ ] Preserve explicit `cwd` behavior and `--dir` default semantics.
- [ ] Capture stdout/stderr with bounded buffers.
- [ ] Return exit code and exact result envelope.
- [ ] Preserve non-zero exit behavior.
- [ ] Preserve command-not-found and spawn-error behavior.
- [ ] Implement timeout and process termination.
- [ ] Add regression tests for success, failure, empty output, non-zero exit, cwd, arguments containing spaces/shell metacharacters, timeout, oversized output, and concurrent execution limits.
- [ ] Verify no shell interpolation is introduced accidentally.

### Phase 5 — PID/lifecycle parity and hardening — [ ] TODO

- [ ] Port atomic exclusive pidfile acquisition.
- [ ] Port stale-lock detection/recovery.
- [ ] Port second-instance rejection.
- [ ] Implement `stop --port <port>` for each supported OS.
- [ ] Handle SIGINT/SIGTERM cleanly.
- [ ] Ensure shutdown does not delete another process's pidfile.
- [ ] Validate pidfile path/ownership against symlink/path-race scenarios.
- [ ] Add race/lifecycle integration tests where practical.
- [ ] Test repeated start/stop cycles and stale-pid recovery.

### Phase 6 — Frontend E2E parity — [ ] TODO

- [ ] Build the Rust binary.
- [ ] Start it on a local test port.
- [ ] Pair from the existing Nuxt UI without changing frontend code.
- [ ] Establish the browser WebSocket connection.
- [ ] Execute a terminal command through the existing local-terminal flow.
- [ ] Verify stdout/stderr/exit code are rendered exactly as before.
- [ ] Verify failure and timeout behavior.
- [ ] Verify revoke/disconnect behavior.
- [ ] Verify wrong/missing Origin is rejected in a browser-like request.
- [ ] Verify no hosted-server terminal data path is introduced.
- [ ] Record the exact tested browser/build/runtime versions.
- [ ] Run the E2E test against the **Rust binary, not a mocked server**.

### Phase 7 — Remove Node relay implementation — [ ] TODO

Only after Phase 6 proves parity:

- [ ] Delete `packages/relay-agent/src/*`.
- [ ] Delete `packages/relay-agent/bin/cli.mjs`.
- [ ] Delete `packages/relay-agent/bin/pidfile.mjs`.
- [ ] Delete `packages/relay-agent/build.mjs`.
- [ ] Remove `packages/relay-agent/package.json` if the package is no longer needed; otherwise reduce it to non-runtime metadata only after confirming the monorepo package manager/release flow does not require it.
- [ ] Remove all relay-agent-specific Node dependencies (`ws`, `execa`, `esbuild`, `@types/*` that become unused).
- [ ] Remove `@yao-pkg/pkg` from the entire monorepo.
- [ ] Remove obsolete Node build scripts and references.
- [ ] Ensure no relay-agent runtime entrypoint remains in JS/TS.

### Phase 8 — Rust-native release pipeline — [ ] TODO

Rewrite `.github/workflows/release-relay-agent.yml` to build Rust artifacts directly.

- [ ] Remove pnpm/Node installation from the relay-agent release job.
- [ ] Install/pin the repository Rust toolchain consistently with `rust-toolchain.toml` / Plan 027 policy.
- [ ] Build `relay-agent` with Cargo in release mode.
- [ ] Produce Linux x64, macOS x64, macOS arm64, and Windows x64 artifacts unless the repository's supported matrix is intentionally changed with evidence.
- [ ] Use native/appropriate cross-compilation or dedicated matrix runners; do not assume an Ubuntu host can transparently produce every target.
- [ ] Rename artifacts to the stable `relay-agent-*` names expected by the Nuxt download flow.
- [ ] Generate checksums.
- [ ] Verify artifacts are standalone and do not require Node.js.
- [ ] Publish release assets from the Rust build output.
- [ ] Ensure Settings → Local Terminal download URLs still resolve without frontend changes.
- [ ] Run the release workflow from a disposable tag before marking this phase complete.
- [ ] Verify each published artifact's target/architecture metadata before release.
- [ ] Verify checksums from a clean machine/checkout rather than only trusting the build workspace.

### Phase 9 — Binary size, supply chain, and production hardening — [ ] TODO

- [ ] Measure each release artifact size.
- [ ] Benchmark `opt-level`/LTO choices and keep the smallest acceptable release configuration based on evidence.
- [ ] Verify no unnecessary Tokio/Axum features are enabled.
- [ ] Verify symbols/debug data are stripped from release artifacts where appropriate.
- [ ] Verify startup time and basic resource usage are acceptable.
- [ ] Verify no Node/V8/libnode dependency is embedded or required.
- [ ] Run repository-approved dependency vulnerability/license checks (`cargo audit` / `cargo deny` or the project's equivalent) and resolve or explicitly accept findings.
- [ ] Commit and review `Cargo.lock` changes as part of the migration; do not rely on an unconstrained dependency graph in release CI.
- [ ] Verify release artifacts from clean checkouts and pinned toolchains.
- [ ] Generate a machine-readable artifact manifest containing target, version/commit, SHA-256, and byte size.
- [ ] If repository release policy supports signing/provenance, sign or attest release artifacts and verify the published provenance.

### Phase 10 — Final removal and closeout — [ ] TODO

- [ ] Repository-wide search proves no `@yao-pkg/pkg` dependency remains.
- [ ] Repository-wide search proves no relay-agent executable JS/TS entrypoint remains.
- [ ] `pnpm install` no longer installs relay-agent-only runtime/build dependencies.
- [ ] `cargo build --release --bin relay-agent` succeeds from a clean checkout.
- [ ] `relay-agent --help` works without Node.js.
- [ ] `relay-agent stop --port <port>` works without Node.js.
- [ ] Full Rust test suite passes.
- [ ] Frontend E2E parity passes without frontend source changes.
- [ ] Security/resource-limit regression suite passes.
- [ ] Release workflow passes for all supported targets.
- [ ] Published binaries are manually smoke-tested from GitHub Release assets.
- [ ] Checksums and artifact manifest verify from a clean environment.
- [ ] Plan 028 is marked `COMPLETED` only after all artifact verification is done.
- [ ] `.agents/plans/README.md` moves Plan 028 from In Flight to Completed with final PR/release evidence.

## Test strategy

### Unit tests

Cover pure logic independently from networking/process execution:

- CLI/default configuration.
- Origin/Host validation.
- exact Origin comparison and missing-Origin rejection.
- pairing token entropy/expiry/reuse.
- credential lookup/revocation and concurrent race behavior.
- command payload parsing.
- result serialization.
- resource-limit configuration.
- pidfile path/ownership logic.

### Integration tests

Use local ephemeral HTTP/WebSocket servers only. No public internet dependency.

Required cases:

- health success.
- pair success.
- invalid token.
- expired token.
- reused token.
- concurrent pair race: exactly one success.
- revoke success/failure.
- missing/wrong Origin.
- missing/wrong Host.
- exact Origin mismatch cases that differ only by scheme/host/port/case/trailing slash as applicable to the frozen contract.
- valid WebSocket credential.
- invalid/revoked WebSocket credential.
- URL-encoded credential.
- oversized HTTP request.
- oversized WebSocket message.
- exec success.
- exec non-zero exit.
- command-not-found.
- stdout/stderr capture.
- explicit cwd.
- argument boundary preservation.
- timeout and child termination.
- output limit enforcement.
- concurrent execution limit.
- malformed JSON.
- unknown message type.
- second instance rejection.
- stale pidfile recovery.
- pidfile ownership/symlink safety where supported by the platform.
- clean stop.
- repeated start/stop cycles.

### Security regression tests

The test suite must explicitly prove:

```text
missing Origin             → reject
wrong Origin               → reject
wildcard Origin            → reject
missing Host               → reject
wrong Host                 → reject
invalid credential         → reject
revoked credential         → reject
reused pairing token       → reject
racing pairing requests    → one success only
oversized message         → bounded rejection
oversized output          → bounded rejection
execution timeout         → process cleanup
```

No security test may rely on a hidden production bypass, debug-only environment variable, or relaxed release configuration.

### Frontend E2E

The strongest parity gate is the real Nuxt UI consuming the Rust binary without a frontend code change. It must prove:

```text
Nuxt UI
  ↓
POST /pair
  ↓
sessionCredential
  ↓
WebSocket ?credential=...
  ↓
{ type: "exec", ... }
  ↓
Rust relay-agent
  ↓
tokio::process::Command
  ↓
{ type: "exec_result", ... }
  ↓
Nuxt UI
```

The E2E harness must start/stop the real binary and use local-only fixtures. A passing mock-server test is not sufficient for the final gate.

## CI gates

The PR is not merge-ready until all applicable checks are green:

- [ ] `cargo fmt --check`.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] `cargo test --workspace`.
- [ ] Relay-agent integration tests.
- [ ] Security/resource-limit regression tests.
- [ ] Frontend/E2E parity tests against the real Rust binary.
- [ ] Repository-wide `@yao-pkg/pkg` absence check.
- [ ] Repository-wide JS/TS relay-agent executable absence check.
- [ ] Release build for every supported target.
- [ ] Release artifact smoke test.
- [ ] Artifact naming/checksum verification.
- [ ] Node.js-independent runtime verification.
- [ ] Dependency vulnerability/license policy check.
- [ ] No unrelated Nuxt regressions.

## Definition of Done

Plan 028 is **CLOSED** only when:

- [ ] `relay-agent` is implemented entirely in Rust.
- [ ] The binary is the sole relay-agent runtime entrypoint.
- [ ] Nuxt requires no functional/source changes to pair/connect/execute commands.
- [ ] HTTP and WebSocket contracts are strictly parity-tested.
- [ ] Origin/Host security remains fail-closed.
- [ ] Pairing is single-use, race-safe, and session credentials are revocable/expiry-bound according to the frozen contract.
- [ ] Request/message/output/concurrency limits are enforced and tested.
- [ ] Command execution uses Tokio and preserves stdout/stderr/exit-code/error semantics.
- [ ] Timeout terminates the child/process tree appropriately and does not block the async runtime.
- [ ] PID/stop/singleton lifecycle behavior is preserved and race-safe.
- [ ] Node.js/TypeScript relay-agent source and build scripts are removed.
- [ ] `@yao-pkg/pkg` is removed from the monorepo.
- [ ] Release CI builds native Rust binaries directly.
- [ ] Supported release assets are standalone and Node-free.
- [ ] Binary sizes are measured and documented.
- [ ] Dependency/security policy checks pass.
- [ ] Full CI is green.
- [ ] Published release binaries are smoke-tested end-to-end.
- [ ] Documentation and Plan 028 match the final implementation.

## Rollback

If the Rust agent fails parity or release verification, keep Plan 028 `IN FLIGHT` and do not remove the Node implementation until the Rust binary passes the full contract/E2E gate. The previous Node/pkg release remains the rollback artifact until the Rust release is proven from actual published binaries.

If a Rust release is published and later fails artifact verification, restore the previous known-good relay-agent release where practical, fix the Rust implementation, republish, and repeat the complete artifact smoke test.

## Evidence log

Record final evidence here as the work progresses:

- Contract inventory: `[ ]`.
- Threat model/resource limits: `[ ]`.
- Rust implementation: `[ ]`.
- Security regression suite: `[ ]`.
- Frontend E2E parity: `[ ]`.
- Node source removal: `[ ]`.
- `@yao-pkg/pkg` removal: `[ ]`.
- Release workflow migration: `[ ]`.
- Dependency/security policy check: `[ ]`.
- Published binary smoke tests: `[ ]`.
- Artifact manifest/checksum verification: `[ ]`.
- Final CI run: `[ ]`.
- Final release/tag: `[ ]`.
