# 026 — Relay agent: browser-to-localhost bridge, no internet in the data path

**Status: closed, all 9 phases shipped.** The user's own manual verification pass happened for real across this plan's later sessions — pairing (including over a Tailscale IP, which surfaced and fixed the `crypto.randomUUID` secure-context bug), a real chat-driven `local_terminal` call (which surfaced and fixed the Phase 9 approval-bypass bug), and real CLI start/stop usage (which surfaced and fixed the Phase 9 pidfile race). Two `relay-agent-v*` releases have been published (`v0.0.1-beta`, `v0.0.2-beta`) — see Phase 7. Nothing known outstanding; the only remaining item is republishing a release once the Phase 9 fixes (approval race, pidfile race) are merged, since the last published binary predates them.

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

### Phase 8 — Removed the workspace-directory jail; `local_terminal` is now automatic once paired [DONE]

Two deliberate, user-directed reversals of earlier decisions in this same plan:

- **No more server-side terminal at all.** The `native.terminal` tool (workspace-sandboxed, server-side, from plan 021 — kept alive in both chat mode and agent mode through Phase 7) was removed entirely, in both modes. `local_terminal` (relay-agent, runs on the user's own machine) is now the *only* shell-execution path anywhere in this app — this server has no code path left that spawns a shell itself. Removed: `server/utils/exec-guard.ts` (dead once nothing called it), the `terminal` wiring block in `server/api/chat.post.ts`, the always-on read-only wiring in `server/utils/langgraph-tools.ts` (chat mode). `buildWorkspaceSystemPrompt` no longer describes any terminal capability — it's just location context now, since a tool's own description (not the system prompt) is what tells the model what it can do.
- **`local_terminal`'s directory jail was removed.** Originally (Phase 1's decision, "workspace-scoped, not whole-host") the relay-agent CLI restricted every command to a single `--dir` root via `resolveScopedPath`'s symlink-aware boundary check. Re-litigated and reversed: `--dir` is now only the *default* starting `cwd` for a command that doesn't specify its own (defaults to the user's home directory, not `process.cwd()`) — a command's `cwd`, or any path-like argument, can target anywhere the OS user account running the CLI can reach. `packages/relay-agent/src/scope.ts` (the jail implementation) was deleted outright, not just unwired. The controls that remain are: pairing (nothing reaches the agent without a valid session credential) and the per-command chat-side approval gate for anything AI-initiated — manual commands typed into the paired browser's own terminal panel need no approval, same as opening a real terminal. `terminalToolSchema` (`packages/terminal-tool/src/index.ts`, shared with `local_terminal`'s tool definition) gained an optional `cwd` field so the model can actually target a directory; `useConversationChat.ts`'s `handleClientToolCall` now forwards it through to `relayAgent.exec()`.
- **`local_terminal` no longer has a chat Tool Picker toggle.** Redundant with the Settings → Local Terminal page already being where a user manages device pairing — `shared/utils/native-tools.ts`'s entry gained `pickerVisible: false` (kept in the registry so approval-id resolution still works; just not rendered as a checkbox). Availability is now driven server-side by `server/api/chat.post.ts` querying whether the user has any non-revoked row in `user_devices` — present in every agent-mode conversation once true, absent otherwise. Fixed a related latent bug found while touching this: `ChatToolPicker.vue`'s "N tools" count was `conv.enabledToolIds.length` verbatim, which kept counting ids for tools removed from the registry (e.g. the just-deleted `native.terminal`) forever, since nothing retroactively cleans up old conversations' stored `enabledToolIds` — count is now filtered to ids that still resolve to something actually listed.

Known open gap, not yet fixed (found during a design-review conversation, not implemented this phase): if the relay-agent WebSocket drops while a command is mid-flight, `useRelayAgent.ts`'s `pendingExecs` promise is never rejected — `exec()` hangs forever, which (for an AI-initiated call) hangs the whole chat turn with no error surfaced. Needs `onclose`/`onerror` to reject every pending exec, plus probably a client-side timeout as a backstop.

Also added in this phase, prompted by a real "how do I stop this" moment: the CLI previously had no graceful shutdown at all (`Ctrl+C` just killed the process outright, no pidfile, no way to stop a detached instance short of manually finding and `kill`ing the PID). Added `bin/pidfile.mjs` (a pidfile at `os.tmpdir()/relay-agent-<port>.pid`, port-scoped since more than one instance can run at once) plus SIGINT/SIGTERM handlers in `bin/cli.mjs` that call `server.stop()` and clean up the pidfile before exiting, and a `relay-agent stop [--port N]` subcommand that reads the pidfile and sends SIGTERM — self-healing if the pidfile is stale (process already dead/crashed).

### Phase 9 — Fix the approval/execution race; add a per-conversation bypass toggle [DONE]

Found live, in a real session, by watching a chat turn get stuck at "Still working…" forever with no approval modal ever appearing: `local_terminal` (a client-executed tool — no server-side `execute`, see Phase 5) has no `execute` for `streamText` to gate behind approval, so **the SDK's approval mechanism does not block it**. Traced through `node_modules/ai/dist/index.js`: every tool call streams a `tool-input-available` chunk unconditionally (which `useConversationChat.ts`'s `onToolCall` was firing on immediately, with no approval check at all) *and separately* a `tool-approval-request` chunk (which only affects `ChatToolApproval.vue`'s modal/`conv.approvals` bookkeeping) — the two are independent; approval state was never actually consulted before executing. Combined with the already-known "exec() hangs forever on disconnect" gap (Phase 8's note above), the result was: the command could start executing before — or regardless of — whatever the user did in a modal that might not even have rendered yet, and if the local agent wasn't reachable, the whole turn hung with no error.

Fixed:
- **`onToolCall` was removed entirely** — nothing executes from it anymore. The only place `local_terminal` ever runs is a `watch(chat.messages, ..., { immediate: true })` in `useConversationChat.ts` that scans for a `tool-local_terminal` part that has actually reached `state === 'approval-responded'` with `approval.approved === true` — true whether that came from the user genuinely clicking Allow just now, or from a remembered `conv.approvals` decision the server already auto-resolved (both produce the exact same state transition server-side, just in different turns — so one code path correctly covers both, no special-casing "remembered" vs "fresh"). A denied response needs no handling on this end — the SDK resolves `output-denied` internally without ever calling a client tool. `immediate: true` matters: a plain `watch()` only reacts to *changes*, so without it a reopened conversation with an already-approved-but-never-executed call sitting in its loaded history (tab closed mid-flight) would never resume it.
- **`useRelayAgent.ts`'s `pendingExecs` are now rejected**, not left dangling, on both `onclose` and `onerror`, plus a 310s client-side timeout backstop for a connection that goes silent without ever firing either event — `exec()` (and therefore an AI-initiated call awaiting it) now fails with a clear error instead of hanging the chat turn forever.
- **Durable dedup across reloads**, closing a narrow window the `immediate: true` resume logic above otherwise opens: if a command actually finished running on the CLI but the follow-up request persisting its output never completed (network dropped at exactly that moment), the DB row looks identical to "never ran" — a reload would then run it again. `useConversationChat.ts` now marks a `toolCallId` as attempted in `localStorage` (capped at 200 entries) *synchronously, before* ever awaiting `exec()`, so even a mid-flight crash right after that line means a reload will never retry it — accepting that a call which fails for an unrelated setup reason also won't self-heal on reload (the model still sees the error via `addToolOutput`; a real duplicate command run was judged the worse failure mode).
- **Server-side device-check query wrapped in try/catch** (`server/api/chat.post.ts`) — found during this same review: an unhandled failure there (exactly what the missing-migration incident above triggered) crashed the *entire* chat request, including unrelated MCP tools. Now degrades to "no local terminal this turn" instead.
- **New per-conversation "skip approval" toggle**, in `ChatToolPicker.vue`'s Built-in section next to `local_terminal` (still not a visibility/enable toggle — availability stays entirely driven by device-pairing status per Phase 8, per the user's explicit correction that this control is *not* about whether the tool can be used). A `USwitch` bound to `conversation.approvals[NATIVE_LOCAL_TERMINAL_TOOL_ID] === 'always'`: on sets it to `'always'` (every future call in *this* conversation only skips the modal — writes the same `conversations.approvals` column `ChatToolApproval.vue`'s own "Always allow" button already writes, so it's the exact same mechanism, just reachable proactively instead of only after a first prompt), off clears it back to requiring a modal per call. Never a global/user-level setting — turning it on in one conversation has no effect on any other, by construction of where it's stored.

Found live, again, right after shipping the `stop` subcommand: starting a second `relay-agent` instance against a port already held by a live first one (e.g. running the same `nohup ... &` command twice by accident) failed with `EADDRINUSE` as expected, but then `relay-agent stop` reported "No running agent found" even though the first instance was still genuinely up. Root cause — `bin/cli.mjs` called `writePidFile(port)` *before* attempting `server.start()`, not after: the second (failing) process overwrote the first (live) process's pidfile with its own pid, then its failure handler unconditionally deleted "the" pidfile — actually deleting the first instance's entry. Fixed: `writePidFile` now runs only inside `server.start().then(...)`, so a failed start never gets a pidfile of its own to clobber anything with; added `removePidFileIfOwnedByMe` (`bin/pidfile.mjs`) — compares the recorded pid to `process.pid` before ever unlinking — and switched the graceful-shutdown handler to it, so no code path can delete a pidfile it doesn't actually own. Reproduced the exact race manually (two instances against one port, confirmed the pidfile survived the second's failure, confirmed `stop` then found and killed the right one) before considering it fixed.

## Out of scope

- Remote (not-same-machine) access — e.g. accessing your desktop from your phone's browser. That needs the internet relay/tunnel model from the original draft (sshx/Teleport-style) and is a different feature; not built here.
- File transfer / SFTP-style features.
- Multi-browser-tab or multi-user sharing of one local agent.
- Sandboxing of any kind on the relay-agent side (containers, restricted OS user, directory jail) — per Phase 8, this was deliberately removed. The control surface is pairing (network/auth boundary) plus the chat-side per-command approval gate for AI-initiated calls; whoever holds a paired session has the same filesystem reach as the OS user account running the CLI. Run the CLI as an account you're comfortable with that.

## Verification (whole-plan acceptance, beyond per-phase checks above)

- Packet capture / network tab while using the local terminal panel shows zero traffic to any non-`127.0.0.1` destination for PTY I/O — only the existing chat-channel traffic to Singapore for AI-driven command requests/results (which carry command+result text, never a raw terminal stream).
- A second, unrelated webpage open in another tab cannot connect to the local agent's port (wrong `Origin`, no valid token).
- Revoking a device in settings is honored by the browser; the CLI itself needs no network reachability from Singapore to enforce this (confirms constraint: server never touches user hosts).
- `pnpm audit`, `pnpm lint`, `pnpm typecheck` clean.
