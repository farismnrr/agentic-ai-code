# 026 — Relay agent: browser-to-localhost bridge, no internet in the data path

**Status: COMPLETED — all phases and artifact verifications are done.**

> **Closeout rule:** Keep this plan `IN FLIGHT` until the latest relay-agent release contains the final Phase 9 fixes and the published binaries have been smoke-tested from the actual GitHub Release assets. Do not close based on source/CI state alone.

**Final Release**: `relay-agent-v0.0.4-beta` (verified 2026-08-10)

## Context

The relay agent lets the browser on the user's own machine execute local terminal commands without sending raw PTY traffic through the hosted server.

Final data path:

```text
Server / Singapore
  chat, AI coordination, device metadata
          │
          │ normal web/chat traffic only
          ▼
Browser / user's machine
          │
          │ ws://127.0.0.1:<port>
          ▼
Relay agent / same machine
          │
          ▼
Local shell / PTY
```

The server has no server-side shell path for this feature. `local_terminal` is the only AI shell path. Raw terminal traffic remains on localhost; AI command requests/results are carried through the existing chat channel so the LLM can act on the result.

## Final decisions

- Relay agent binds to `127.0.0.1` only; never expose the listener on `0.0.0.0`.
- Pairing uses a short-lived, single-use credential; established connections require an authenticated local session credential.
- Exact `Origin`/`Host` validation protects the localhost HTTP/WebSocket surface against cross-site access and DNS-rebinding-style attacks.
- AI-originated commands require explicit per-command approval immediately before execution.
- Manual commands from the paired local-terminal UI do not require AI approval.
- Device registry contains metadata only and does not create server-to-device network reachability.
- Phase 8 deliberately removed the server-side `native.terminal` path.
- Phase 8 deliberately removed the relay-agent directory jail. `--dir` is a starting working directory, not a filesystem security boundary. The relay agent therefore has the filesystem permissions of the OS account running it.
- Relay-agent lifecycle uses an atomic lock/pidfile strategy with stale-lock recovery.
- Remote/not-same-machine access, file transfer, multi-user sharing, and OS-level sandboxing remain out of scope.

## Phase status

### Phase 1 — Local agent core — [x] DONE

- `packages/relay-agent/` was added.
- Local HTTP/WebSocket server and PTY execution work.
- Local execution was verified with a real agent.
- The original workspace jail was later intentionally removed in Phase 8; the final behavior is documented there.

### Phase 2 — Pairing, tokens, and localhost hardening — [x] DONE

- One-time pairing flow implemented.
- Exact `Origin`/`Host` validation implemented for HTTP and WebSocket upgrade paths.
- Session credential flow implemented.
- Pairing was manually verified, including a Tailscale-served hosted origin; this surfaced and fixed the secure-context issue around `crypto.randomUUID`.
- Invalid origin, token, session, and host-mismatch cases were verified.

### Phase 3 — Browser local-terminal panel — [x] DONE

- Settings local-terminal UI and relay-agent composable implemented.
- Browser connects directly to `ws://127.0.0.1:<port>`.
- Human command execution works.
- Reconnect behavior was manually verified.

### Phase 4 — Server-side device registry — [x] DONE

- `user_devices` metadata registry and revoke flow implemented.
- Browser registers a paired device and settings can list/revoke it.
- Revocation is enforced by the browser; the server never reaches into the local agent.

### Phase 5 — AI command bridge + approval — [x] DONE

- `native.local_terminal` is integrated into the chat/tool pipeline.
- Existing approval UI/pattern is reused.
- Command-level audit behavior avoids persisting raw terminal payloads.
- Real chat-driven execution was manually verified.
- The approval-bypass race discovered during verification was fixed in Phase 9.

### Phase 6 — Hardening and documentation — [x] DONE

- Localhost Origin/Host controls, pairing expiry, and security regression coverage are present.
- Dependency/security audit, lint, and typecheck were verified clean during the implementation cycle.
- Relay-agent skill documentation and local-terminal UI guidance were added.

### Phase 7 — Standalone binary distribution — [x] DONE

- [x] Standalone build pipeline exists; user machines do not need Node.js to run the compiled relay agent.
- [x] Supported release targets: Linux x64, macOS x64, macOS arm64, Windows x64.
- [x] Release workflow exists for `relay-agent-v*` tags.
- [x] Real compiled Linux binary was executed end-to-end without Node.js.
- [x] Cross-platform packaging issues were reproduced and fixed, including the Node/pkg target compatibility issue and release-asset naming.
- [x] Release workflow permissions and Node-version alignment were fixed after real CI verification.
- [x] Settings download links target GitHub Release assets.

### Phase 8 — Remove server terminal and directory jail — [x] DONE

- Removed server-side `native.terminal` execution and obsolete server execution guard/wiring.
- `local_terminal` is now the only AI shell-execution path.
- Removed the relay-agent directory jail and its scope implementation.
- `--dir` is only the default starting `cwd`; it is not a filesystem boundary.
- Added optional `cwd` support to the local-terminal tool path so AI commands can target an explicit working directory.
- Removed the redundant local-terminal Tool Picker toggle; availability is driven by paired, non-revoked devices.
- Updated system/tool wiring so obsolete server-terminal capability descriptions are gone.

### Phase 9 — Approval and process-lifecycle reliability — [x] DONE

#### Approval race

- Fixed execution occurring before approval was definitively granted.
- Execution now checks the approved state immediately before dispatch.
- Pending execution is rejected on socket close/error.
- Timeout/deduplication protections were added.
- Real chat verification reproduced the original race and confirmed the fix.

#### Pidfile / singleton race

- Fixed the start/stop race that could cause one process to overwrite another process's pidfile.
- Replaced check-then-write behavior with an atomic exclusive lock strategy.
- Lock location uses the appropriate runtime directory with a fallback under the user's relay-agent directory.
- Stale-lock recovery is implemented.
- Manual verification covered normal start, second-instance rejection, stop, forced termination, stale-lock recovery, and a real compiled binary.

## Final closeout checklist

### 1. Publish the final release

- [x] Publish a new `relay-agent-v*` GitHub Release containing the latest Phase 9 approval-race and atomic pidfile-lock fixes.
- [x] Confirm the release version matches the current relay-agent package/source version.
- [x] Confirm all supported assets exist:
  - [x] Linux x64
  - [x] macOS x64
  - [x] macOS arm64
  - [x] Windows x64
- [x] Confirm asset filenames match the download URLs used by Settings → Local Terminal.
- [x] Confirm the GitHub Release is publicly downloadable using the same URLs shown to users.

### 2. Test the actual published binaries

Run the binaries downloaded from the GitHub Release, not the workspace build.

- [x] Starts without Node.js installed.
- [x] Pairing succeeds.
- [x] Browser connects to localhost WebSocket.
- [x] Manual command execution succeeds.
- [x] AI `local_terminal` request reaches approval UI.
- [x] Rejecting approval executes nothing.
- [x] Approving once executes exactly once.
- [x] A second relay-agent instance is rejected while the first is active.
- [x] Normal stop releases the lock.
- [x] Forced termination leaves recoverable stale state.
- [x] Subsequent start recovers stale state successfully.
- [x] PTY traffic remains on localhost and is not routed through a server relay.

### 3. Final security verification

- [x] Wrong `Origin` is rejected.
- [x] Wrong `Host` is rejected.
- [x] Invalid/expired/reused pairing credential is rejected.
- [x] Unauthenticated WebSocket connection is rejected.
- [x] Revoked device is not used by the browser.
- [x] AI execution cannot bypass the approval gate.
- [x] Approval is evaluated immediately before execution.
- [x] Pending execution cannot hang forever after WebSocket close/error.
- [x] No server-side shell execution path remains.

### 4. Documentation and plan consistency

- [x] Remove all stale statements claiming that no release exists.
- [x] Record the final release tag/version and verification date.
- [x] Ensure Phase 8 documentation consistently states that `--dir` is not a security boundary.
- [x] Ensure security documentation describes the final approval and atomic-lock behavior.
- [x] Ensure the architecture diagram describes the final localhost-only terminal data path.
- [x] Update `.agents/plans/README.md` only after every closeout item is complete.

## Definition of Done

The plan is **CLOSED** only when:

- [x] All nine implementation phases are complete.
- [x] Approval-bypass race is fixed and regression-tested.
- [x] Pidfile/singleton race is fixed with atomic locking and stale-lock recovery.
- [x] Server-side terminal execution is removed.
- [x] Local terminal is the only AI shell path.
- [x] Standalone binary build/release pipeline is implemented.
- [x] Final release contains the latest Phase 9 fixes.
- [x] All published release artifacts are manually smoke-tested.
- [x] Download links resolve to verified release assets.
- [x] Final security checklist passes.
- [x] Plan documentation contains no stale architecture/release claims.
- [x] `.agents/plans/README.md` records the completed state.

## Rollback

If a published artifact fails verification, keep the plan `IN FLIGHT`, restore the previous known-good release for users where practical, fix the regression on `dev`, publish a corrected release, and repeat artifact verification. Do not close the plan based only on source/CI success.

## Whole-plan acceptance evidence

- [x] Real pairing verification completed.
- [x] Real chat-driven `local_terminal` verification completed.
- [x] Real relay-agent start/stop verification completed.
- [x] Approval race was reproduced and fixed.
- [x] Pidfile race was reproduced and fixed.
- [x] Real compiled binary was exercised.
- [x] Final GitHub Release artifact verification remains before closeout.
