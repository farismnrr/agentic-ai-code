# 028 — Relay Agent: Full Rust Rewrite + MCP Server

**Status: COMPLETED**  
**Last reconciled: 2026-08-12**

## Purpose

Plan 028 replaced the first-generation Node/WebSocket relay with the current standalone Rust MCP relay and established the security/release boundary used by the product today.

This file is now a **completed historical summary**, not an active checklist. Earlier versions contained phase-by-phase snapshots where Phases 15–17 still appeared `IN FLIGHT`/`NOT STARTED` even though the later final-security and E2E closeout had already completed. Those stale intermediate labels are intentionally removed here so the plan cannot contradict its own final status.

For detailed completion evidence, use:

- [Phase 12 — legacy relay removal](028-phase12-legacy-relay-removal.md)
- [Phase 19 — final security boundary, CI integrity, and release verification](028-phase19-final-security.md)
- [Plan 029 — native MCP integration](029-external-mcp-native-mcp-integration.md)
- [Plan 029b — remaining production/live-acceptance hardening](029b-external-mcp-mcp-production-hardening.md)
- [`packages/relay-agent/SKILL.md`](../../packages/relay-agent/SKILL.md) for the current operational contract

## Final architecture

```text
LOCAL
Nuxt / local MCP client
  -> 127.0.0.1 Streamable HTTP
  -> Rust relay-agent
  -> Origin/Host validation
  -> MCP schema validation
  -> local-mode authorization boundary
  -> filesystem/process/resource policy
  -> Rust native tools / approved child processes

REMOTE
remote MCP client
  -> HTTPS
  -> OAuth-protected MCP resource server
  -> issuer/audience/resource/scope validation
  -> MCP schema validation
  -> same capability/filesystem/process policy
  -> approved tool execution environment
```

The remote deployment is separate from the local loopback agent. Publicly binding or port-forwarding the local no-auth listener is not a supported deployment model.

## Final decisions

- `packages/relay-agent` is Rust; the legacy Node relay is not a runtime fallback.
- MCP `2026-07-28` Streamable HTTP is the relay execution protocol.
- Local mode is loopback-only and browser-origin requests fail closed against configured Origin/Host policy.
- Remote mode is an OAuth-protected resource-server deployment, not a public exposure of local mode.
- The relay runs non-root and refuses root startup on the supported platform.
- Bubblewrap is the required Linux execution sandbox; there is no insecure non-Bubblewrap fallback.
- The execution boundary is an explicit `execution_root`, not merely `cwd`/`--dir`.
- Normal read/write coding operations inside the configured boundary are intentional. The security model is containment + authorization, not a superficial denylist of ordinary development commands.
- Privilege-escalation helpers and paths that undermine the boundary are rejected.
- Child execution has server-controlled timeout, output/input limits, process-tree cleanup, environment controls, and concurrency/resource bounds.
- MCP authorization is server-side and independent from tool arguments. Request data cannot grant itself new capabilities or disable guards.
- Network-capable tools retain SSRF/redirect protections and trusted-endpoint policy.
- CI/release checks are fail-closed: warnings, lint failures, audit findings, security-policy violations, and bypass patterns are blockers rather than accepted exceptions.
- Release artifacts are native Rust artifacts built from the reviewed commit.

## Legacy compatibility removal

Completed Phase 12 removed the old compatibility surface:

- `/pair` and `/revoke` pairing flow;
- legacy WebSocket execution;
- `credential=` execution paths;
- `exec` / `exec_result` compatibility protocol;
- compatibility-only session state and helpers;
- obsolete Node relay runtime/build/release references.

The current `/health` route is a liveness endpoint in the Rust transport; it is not a restored legacy execution protocol.

## Security closeout

The final security closeout records the completed boundary across:

- component-aware filesystem containment;
- symlink/hardlink/rename/TOCTOU review;
- non-root identity and privilege-escalation prevention;
- Bubblewrap mount/process/environment containment;
- Docker/runtime-socket threat-model handling;
- server-side MCP scope-to-tool authorization;
- OAuth issuer/resource/audience/signature/expiry/PKCE requirements;
- credential/log redaction;
- deterministic no-bypass CI checks;
- release gating and artifact verification;
- final local/remote E2E verification.

Those details live in [Phase 19](028-phase19-final-security.md), whose checklist and final E2E completion gate are the authoritative closeout evidence.

## Completion criteria — satisfied

- [x] Relay runtime is standalone Rust.
- [x] MCP Streamable HTTP is the sole relay execution protocol.
- [x] Local mode is loopback-only and non-root.
- [x] No supported privilege-escalation path exists.
- [x] Read/write coding operations remain available inside the configured execution boundary.
- [x] Filesystem/process/runtime containment is fail-closed.
- [x] Tool authorization is server-controlled.
- [x] Network/resource/output/process limits are enforced server-side.
- [x] Remote mode uses HTTPS + OAuth resource-server authorization.
- [x] Credentials/tokens are kept out of logs, URLs, tool arguments, and release artifacts.
- [x] No legacy Node relay or JavaScript execution fallback remains.
- [x] Rust formatting/compiler/Clippy/audit/security gates are enforced without suppression-based bypasses.
- [x] Final E2E and release verification completed.

## Rollback rule

A regression must be fixed by restoring the last known-good Rust artifact or reverting the relevant reviewed change. Do not reintroduce the deleted Node/WebSocket relay, weaken Bubblewrap/authorization, or add a silent fallback merely to restore availability.

## Current status boundary

Plan 028 itself is complete. Remaining live-client/production acceptance work is tracked separately in [Plan 029b](029b-external-mcp-mcp-production-hardening.md); do not reopen this historical rewrite plan for that work.
