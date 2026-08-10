# 026 — Relay agent: browser-to-localhost bridge, no internet in the data path

## Context

The web app runs on a server in Singapore. Today the only shell the AI can touch is the sandboxed workspace directory on that server (plan 021's `terminal-tool`). The user wants the AI/browser to be able to act on **the user's own machine** instead — install packages, run commands in a workspace folder on their laptop — without:

1. The AI sandbox (server-side) ever touching the user's host directly.
2. **Terminal traffic ever leaving the user's machine over the internet.** ← hard requirement, stated explicitly. The first draft of this plan routed CLI↔browser through a hosted relay over `wss://`; that's wrong for this requirement and has been dropped.

### The three roles, explicitly

- **Server** — hosted in Singapore. Serves the web app, runs the LLM/chat pipeline, stores accounts/settings/device metadata. Never has, and never needs, network reachability into any user's machine.
- **Client** — the user's browser, opened on their own laptop. Where that browser *reaches Singapore from* is irrelevant to this feature — direct connection, office network, VPN, doesn't matter, because the browser talks to Singapore for chat/page/approvals only. That path carries zero terminal bytes.
- **Terminal (the "relay agent")** — a CLI the user installs and runs on that same laptop. The user calls it a "relay" because it's the thing standing between the web UI and their real shell, but it does not relay anything over the internet — it binds `127.0.0.1` only and is reached exclusively via browser loopback. **This is why VPN or network path never changes terminal behavior: the terminal connection isn't routed through any network path at all, VPN or otherwise — it's loopback on the same machine.**

### Revised architecture: browser talks to localhost, not to the internet

The browser tab is *itself running on the user's machine*, even though the page was served from Singapore. So the data path doesn't need to leave the machine at all:

```
                                   normal internet (any path: direct, VPN, whatever — doesn't matter)
[Server — Singapore]  <===================================================================>  [Client — browser, on user's laptop]
   web app, chat/LLM,                 page load, chat messages, AI-run-command requests                    |
   device registry (metadata)          + approvals only — never raw terminal bytes                          |
                                                                                                 ws://127.0.0.1:<port>
                                                                                                 loopback — cannot leave the machine,
                                                                                                 regardless of what network the browser used
                                                                                                 to reach the server above
                                                                                                               |
                                                                                          [Terminal — relay agent CLI, same laptop]
                                                                                          binds 127.0.0.1 only, spawns PTY,
                                                                                          scoped to one workspace folder
```

- The **relay agent** (`packages/relay-agent/`) binds a WebSocket+HTTP server to `127.0.0.1` only (never `0.0.0.0`) and spawns/manages PTYs. User must install and run it — there is no terminal feature without it, by design (constraint: "user wajib install dlu sebagai relay").
- The **browser tab**, when the user opens the "local terminal" panel, connects directly to that local port. This connection never touches Singapore, never touches any relay — it's loopback traffic, physically incapable of leaving the machine, independent of whatever route (VPN or not) the same browser uses for its normal Singapore traffic.
- Singapore's server is only involved in two things that are *not* PTY bytes: (a) telling the browser "the AI wants to run `<command>`" and receiving the result back, so the LLM can see output — routed through the **existing chat channel**, not a new tunnel; (b) a lightweight device registry for pairing/revocation UI (metadata only: device name, paired-at, last-seen, revoked — never command content).
- This is why the "reverse tunnel to a relay" research from the previous draft (sshx/Teleport) no longer applies to the data plane — it solves "browser and target machine are different machines," which isn't this use case. It still stays reference material if a genuinely remote (not-same-machine) mode is ever wanted later, but is explicitly **out of scope** now.

### Threats specific to "a webpage talks to a localhost server"

This shape has known pitfalls, all addressed below:

- **Any website can probe/attack localhost ports** ("localhost is not a security boundary" by default) — mitigated with a pairing token, strict CORS to one exact origin, and `Origin`/`Host` header checks on every request including the WebSocket upgrade.
- **DNS rebinding**: an attacker-controlled domain can resolve to `127.0.0.1` after the browser's same-origin check passes, then hit the local server as if it were `localhost`. Mitigated by checking the `Host` header equals `127.0.0.1:<port>` or `localhost:<port>` literally, and the `Origin` header equals the exact hosted app origin — reject anything else.
- **Other tabs/extensions on the same browser** could also reach `127.0.0.1:<port>` if the port is discoverable and unauthenticated. Mitigated by requiring the pairing token on every connection (not just first) and keeping the port itself non-guessable-but-not-secret (defense in depth, not the primary control — the token is).

## Decisions

- **New workspace package `packages/relay-agent/`** — the CLI the user installs and runs on their own machine (`npx @ai-code/relay-agent start`), same distribution convention as `terminal-tool`/`curl-tool` (`package.json`, `bin/cli.mjs`, `SKILL.md`).
- **Workspace-scoped, not whole-host**: on first run the CLI asks for (or takes as `--dir`) a root folder — the equivalent of `resolveWorkspacePath` but enforced client-side. Every command/PTY the CLI spawns gets that folder as `cwd`; the agent refuses to `cd`/resolve paths outside it (same symlink-aware realpath check pattern already used server-side, ported to the CLI). This directly answers "misal gw kasih workspace A" — workspace A is the boundary, not the user's whole filesystem, by default. An explicit `--unrestricted` flag can widen it later if the user wants, but the default is scoped.
- **Auth model has two independent layers, both required**:
  1. **Pairing token** — one-time, short-lived, generated by `relay-agent start` (printed to the terminal + shown as a QR/short code), entered into the web UI once. This is the only thing standing between "any webpage" and "your local shell," so it must never be guessable and must expire (~5 min, single use).
  2. **Session credential** — after pairing, the CLI issues the browser a long-lived-but-revocable local session key (stored in the browser's `localStorage` for the hosted app's origin only, never sent to Singapore). Reconnects use this, not the pairing token again.
- **AI-driven commands require per-command approval** — reusing plan 021's `ChatToolApproval.vue` pattern exactly, not inventing a new one. When the LLM wants to run something on the user's paired local agent, the request flows: server → chat channel → browser shows the existing approval modal → on approve, browser sends the command over the *localhost* WS to the CLI → result comes back to browser → browser reports the result back to the server over the *chat* channel (not the localhost socket) so the LLM sees it. Manual human typing into the local terminal panel needs no approval — that's the user acting on their own machine directly, same trust level as opening a normal terminal.
- **Command-level audit, no payload**: the CLI (locally) and the browser (before forwarding to chat) both log `{command, exitCode, timestamp}` — never full stdout/stderr — to make "what did the AI run on my machine" reviewable without turning the audit log into a second copy of the terminal transcript.
- **Server-side device registry stays minimal**: `user_devices(id, userId, name, pairedAt, lastSeenAt, revokedAt)` — written once per pairing (browser reports it after successful local pairing), read by the settings page and by the browser itself (to refuse reconnecting to a revoked device even though revocation can't reach the CLI directly, since the CLI was never told about Singapore).

## Phases

Small, independently shippable/testable steps — implement and verify each before starting the next.

### Phase 1 — Local agent core (no auth yet, no browser integration) [DONE]
- `packages/relay-agent/`: CLI that binds `127.0.0.1:<port>` (pick a fixed default, e.g. `47821`, configurable via `--port`), spawns a PTY via `node-pty` (or `execa` for one-shot commands, matching `terminal-tool`'s approach — decide PTY vs exec-per-command here), scoped to `--dir` (default: cwd where `start` was run).
- No pairing/token yet — just prove the local WS server can spawn a shell and stream I/O to a bare test client (e.g. a local HTML file opened directly, or `wscat`).
- Verification: `relay-agent start --dir ./somewhere`, connect a local WS test client, run `ls`, confirm output and confirm a `cd ../../` or absolute-path-outside-dir attempt is rejected by the workspace-scope check.

### Phase 2 — Pairing, tokens, and localhost-attack hardening [DONE]
- Implement the one-time pairing token (printed on `start`), the CORS allowlist (exact hosted origin, no wildcard), and `Origin`/`Host` header validation on both the HTTP pairing endpoint and the WS upgrade.
- Implement the long-lived session credential issuance after successful pairing.
- Verification: a request/connection from a disallowed `Origin` (simulate with `curl -H "Origin: https://evil.example"` and a manual WS client) is rejected; a request with the correct `Origin` but wrong/expired/reused pairing token is rejected; a paired session credential works across a CLI restart (same port, credential persisted CLI-side) but a mismatched `Host` header (rebinding simulation) is rejected.

### Phase 3 — Browser-side local terminal panel [DONE]
- New UI (`app/pages/settings/local-terminal.vue` + `app/composables/useRelayAgent.ts`) that: shows pairing instructions/token input, connects to `ws://127.0.0.1:<port>` once paired, renders execution output, sends commands directly.
- No AI involvement yet — this phase is purely "human types in browser, local machine executes," to validate the localhost bridge end-to-end in the real UI.
- Verification: manual — pair a real local agent, type commands, confirm they run in the scoped workspace folder and nowhere else; confirm closing/reopening the tab reconnects using the stored session credential without re-pairing.

### Phase 4 — Server-side device registry (metadata only) [DONE]
- `user_devices` table + minimal API (`server/api/devices/index.ts`, `server/api/devices/[id]/revoke.post.ts`): browser registers a device after local pairing succeeds (`POST /api/devices` with device name + fingerprint), settings page lists devices with "Revoke".
- Revoke sets `revokedAt`; browser checks this and refuses to use a stored session credential for a revoked device.
- Verification: revoking a device in settings makes the browser stop using its stored credential on next check, without any server-side ability to reach into the local agent.

### Phase 5 — AI-driven command bridge + approval gate [DONE]
- Extend the chat pipeline (`server/api/chat.post.ts`, `shared/utils/native-tools.ts`) with `native.local_terminal` tool registration and approval gating via `ChatToolApproval.vue`.
- Command-level audit logging and approval reuse logic aligned with plan 021's existing tool approval patterns.
- Verification: tool availability controlled by native tools selection and approval gate.

### Phase 6 — Hardening pass and docs [DONE]
- Hardening review: workspace-scope traversal validation, CORS/Origin/Host header checks, single-use token expiry, zero-vulnerability dependencies verified via `pnpm audit`.
- Added documentation in `packages/relay-agent/SKILL.md` and UI guidance on `app/pages/settings/local-terminal.vue`.

### Phase 7 — Standalone binary distribution (no Node.js required on the user's machine) [DONE]
- Added esbuild bundle step (`build.mjs`) and `@yao-pkg/pkg` cross-compilation target matrix for Linux x64, macOS x64/arm64, and Windows x64.
- Added GitHub Actions workflow `.github/workflows/release-relay-agent.yml` to build and publish binaries on `relay-agent-v*` tag push — **not yet triggered**; no tag has been pushed and no GitHub Release exists yet, so the UI's download links 404 until the first real release.
- Updated UI (`app/pages/settings/local-terminal.vue`) with OS auto-detection, direct download buttons pointing to GitHub Release asset URLs, and an explicit `--origin` value derived from `useRequestURL()` in the paired-CLI example command (was previously missing entirely, which would have broken pairing on any deployment other than the CLI's local-dev default).
- End-to-end verified locally (not just read from logs): ran the real `compile` step, executed the resulting standalone Linux binary directly (no `node`/`npx` involved), paired it over real HTTP, and ran a real command over the real WebSocket exec channel — confirmed working before trusting the CI workflow to do the same.
- Two real bugs found and fixed only by actually running the compile, not by reading the tool's docs:
  - `@yao-pkg/pkg`'s remote prebuilt-binary cache had no entry for `node20`'s current patch version (`v20.20.2`) on any platform — it silently fell back to compiling Node.js from source, which hard-fails outright for `macos`/`win` targets from a Linux runner (`Error! Not able to build for 'macos' here, only for 'linux'` — source builds can't cross-compile OS, only prebuilt-binary fetches can). Probed cache coverage across `node16`/`node18`/`node20`/`node22`/`node24` × all 4 targets with `pkg-fetch -t` (fetch-only, no build); `node22`/`node24` had full 4-platform coverage, `node18`/`node20` did not. `package.json`'s `compile` script is now pinned to `node22` — see the verification method documented in `packages/relay-agent/SKILL.md` for re-checking this before ever bumping the version.
  - The release workflow's rename step assumed pkg's output filenames were `bundle-linux`/`bundle-win.exe`; the real names (confirmed by inspecting `dist/bin/` after a real compile) are `bundle-linux-x64`/`bundle-win-x64.exe` — the workflow would have failed at that step on its first real run. Fixed.
- Also fixed: the dead, never-read `"pkg"` config block in `package.json` (the `compile` script already passes `--targets`/`--out-path` as explicit CLI flags, so that block was pure noise); the release workflow's `node-version: 20` → `22` to match `ci.yml`'s own Node version.
- Not yet done: pushing a `relay-agent-v*` tag to actually produce a GitHub Release (deliberately left to the user — publishing is a one-way action). Until that happens, the download buttons in Settings → Local Terminal are dead links.

## Out of scope

- Remote (not-same-machine) access — e.g. accessing your desktop from your phone's browser. That needs the internet relay/tunnel model from the original draft (sshx/Teleport-style) and is a different feature; not built here.
- File transfer / SFTP-style features.
- Multi-browser-tab or multi-user sharing of one local agent.
- Sandboxing beyond the workspace-directory scope (containers, restricted OS user) — the directory boundary plus per-command approval for AI-driven actions is the control surface for v1; the user's own OS-level permissions are the rest of the boundary, same as running a normal terminal.

## Verification (whole-plan acceptance, beyond per-phase checks above)

- Packet capture / network tab while using the local terminal panel shows zero traffic to any non-`127.0.0.1` destination for PTY I/O — only the existing chat-channel traffic to Singapore for AI-driven command requests/results (which carry command+result text, never a raw terminal stream).
- A second, unrelated webpage open in another tab cannot connect to the local agent's port (wrong `Origin`, no valid token).
- Revoking a device in settings is honored by the browser; the CLI itself needs no network reachability from Singapore to enforce this (confirms constraint: server never touches user hosts).
- `pnpm audit`, `pnpm lint`, `pnpm typecheck` clean.
