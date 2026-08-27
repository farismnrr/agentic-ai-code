# Plan 051 — Unified MCP Settings and Local Relay UX

**Status:** IMPLEMENTED — PR #171 MERGED; POST-MERGE UX REMEDIATION VERIFIED; RECREATE PENDING
**Created:** 2026-08-27

## Execution status — 2026-08-27

The main Plan-051 implementation merged to `main` through PR #171. A browser review immediately after merge exposed one product-model mismatch: the Local relay was rendered as if already added and settings probed loopback automatically, so a fresh user saw a phantom connection and unsolicited browser CORS failures before choosing any connector. The post-merge remediation on `fix/051-mcp-local-connection-ux` corrects that behavior without changing the Rust relay/security contract.

Verified locally after remediation:

- fresh browsers show a single **No MCP connections yet** state; **Add MCP** opens the Local relay / Remote MCP choice first;
- Local relay is added only after an explicit modern-MCP verification and is persisted only in browser storage;
- Local relay and remote servers both expose a consistent **Remove connection** action; removing Local relay clears the browser-local capability state but does not stop the external process;
- Settings does not probe `127.0.0.1` before a user adds/checks the Local relay, preventing the unsolicited CORS requests seen in the initial browser review;
- manual preflight against the currently running `127.0.0.1:47821` relay permits the required MCP headers/methods but does not return `Access-Control-Allow-Origin` for `http://100.99.88.53:3333`, confirming that runtime was started for a different exact Origin; the UI now tells the user to restart it with the generated `--origin` command instead of weakening CORS;
- stale `native.local_terminal` conversation IDs cannot re-enable Agent Mode or client execution after the browser-local connection is removed;
- `pnpm guardrail` — PASS (24/24 web tests; Rust correctly skipped because no Rust/native source changed);
- `NUXT_SESSION_PASSWORD=<build-only-value> pnpm build` — PASS.

Authenticated visual/mobile interaction and a live third-party remote MCP scan are not claimed because those fixtures are unavailable through the current execution tools. App-container recreation remains pending because the MCP coding sandbox intentionally cannot read the repository `.env` required by Docker Compose.

## Goal

Replace the separate **Local Terminal** settings surface with one polished, industry-standard **Settings → MCP** connection-management experience. Remote MCP servers and the first-party browser-local relay should be managed from the same information architecture while preserving their different trust, persistence, and execution boundaries.

The product target is a clean connector manager similar in interaction model to modern MCP/plugin setup flows: users open one MCP page, add or manage a connection in a focused overlay, verify/discover capabilities before relying on it, and see clear connection state and tool counts without being exposed to an unrelated terminal console.

## Success criteria

Plan 051 is complete only when:

1. **Local Terminal** no longer appears as a top-level Settings tab;
2. `/settings/mcp` is the single user-facing management surface for local relay and remote MCP connections;
3. the local relay is presented as a first-party local connection, but is never mis-modeled as a server-side `http://127.0.0.1` MCP row;
4. remote MCP creation follows a verify-before-save flow: configure → scan/test → review discovered tools → create;
5. remote MCP configuration is validated server-side through the same SSRF-safe client path used at runtime, and server-owned fields such as status/tools are not writable by the browser;
6. unsupported `stdio` execution is no longer offered as a normal creation choice while existing legacy rows remain fail-closed and manageable;
7. local relay setup retains the useful install/start command guidance and terminal-network opt-in, but removes the arbitrary shell runner/history from Settings;
8. Agent Mode and the chat tool picker continue to depend on the real browser-local relay connection without changing the Rust execution/security contract;
9. the UI is responsive, keyboard-accessible, semantically themed, concise, and visually consistent with the existing Nuxt UI dashboard;
10. relevant web tests, lint, typecheck, build, browser verification, and `pnpm guardrail` pass without unrelated Rust validation unless a real shared Rust contract is changed;
11. repository docs/agent memory remain truthful and the change is delivered through the repository-required short-lived branch and PR flow.

## Scope

### In scope

- Settings navigation cleanup;
- unified MCP/local-relay connection presentation;
- a reusable create/manage overlay patterned after modern MCP/plugin setup UX;
- remote MCP draft scan/test before persistence;
- remote MCP create/edit/recheck/remove flows;
- removal of user-facing `stdio` creation because server execution deliberately rejects it;
- server-side ownership of discovered tools and connection status;
- first-party local relay setup, live connection check, and device-local port preference;
- local relay install/start command generation, including the existing `--allow-terminal-network` opt-in;
- chat tool-picker/settings-link integration;
- removal of Settings-only terminal job/history code that becomes dead after the old page is removed;
- legacy `/settings/local-terminal` navigation compatibility through a Nuxt-native redirect;
- feature-named web tests under top-level `test/`;
- responsive/browser/accessibility review and documentation closeout.

### Out of scope

- changing the Rust relay MCP protocol, Bubblewrap boundary, approval policy, or terminal execution semantics;
- moving browser-local relay execution into Nitro/server-side shell execution;
- storing the local relay as a generic remote MCP URL in Postgres;
- adding arbitrary third-party OAuth/API-key credential storage that the current product does not support;
- enabling server-side `stdio` process spawning;
- keeping an interactive arbitrary shell console inside Settings;
- redesigning unrelated Settings pages or the general chat layout;
- introducing a new UI/test framework, plan-numbered verification script, or GitHub Actions workflow.

## Verified current state

Verified from `main` on 2026-08-27 before this plan was created:

- `app/pages/settings.vue` exposes six settings tabs, including a dedicated `Local Terminal` route.
- `app/pages/settings/local-terminal.vue` is a 326-line combined setup/status/terminal-console page. It downloads the Linux relay, generates foreground/background launch commands, toggles the `--allow-terminal-network` command flag, checks loopback health, and runs arbitrary terminal jobs with output/history.
- `app/pages/settings/mcp.vue` is a separate 325-line MCP server manager. It persists remote server rows first, then requires a second manual **Test connection** action to discover tools.
- `app/pages/settings/mcp.vue` currently offers `http`, `sse`, and `stdio`, but `server/infrastructure/mcp/client.ts` deliberately rejects `stdio` because executing a user-controlled command would spawn a server-side child process.
- `server/api/mcp-servers/index.post.ts` accepts a broadly typed transport and creates a disconnected row without proving the endpoint.
- `server/api/mcp-servers/[id].put.ts` currently accepts browser-supplied `status` and `tools`, even though those facts should be server-owned.
- `server/infrastructure/mcp/test-server.ts` performs real MCP discovery through `createMcpClient()`, maps tools to stable server-qualified IDs, persists status/tools on success, and marks the row `error` on failure.
- `server/infrastructure/mcp/client.ts` already applies SSRF-safe outbound fetching and has a special exact first-party remote path that keeps its bearer token private and owner-bound.
- `app/composables/useRelayAgent.ts` owns the browser-local loopback relay at default port `47821`; its connection state is shared through Nuxt `useState()` and its MCP requests go directly from the browser to `127.0.0.1`.
- `ChatConfigControls.vue` makes Agent Mode available only when `native.local_terminal` is enabled and the local relay is connected.
- `ChatToolPicker.vue` exposes the local terminal as a native capability and links disconnected users directly to `/settings/local-terminal`.
- `startJob()`, `getJob()`, and `cancelJob()` in `useRelayAgent.ts` are used by the Settings terminal console and are not required by the normal conversation execution path.
- remote MCP server rows are user-scoped in Postgres, while local loopback relay identity is inherently device/browser-specific; these two persistence models must not be conflated merely to unify the UI.
- repository policy requires Nuxt-native patterns, Nuxt UI components, semantic colors, feature-named tests under `test/`, stack-proportional validation, `pnpm guardrail`, short-lived branches, and PR delivery. The repository intentionally has no CI.

## Constraints and architecture decisions

### AD-001 — One management surface, two runtime owners

Settings presents one **MCP connections** product surface, but implementation preserves two backends:

- **Local relay:** browser/device-scoped, loopback-only, live status checked from the browser, never persisted as a remote Postgres MCP URL.
- **Remote MCP:** user-scoped Postgres configuration, server-side outbound connection, SSRF policy, private first-party runtime credentials where applicable.

UI unification must not erase this security/runtime distinction.

### AD-002 — Local relay is a built-in connection, not a fake remote server

The local relay appears alongside remote connections as a first-party connection card and uses the same visual vocabulary/manage overlay. It does not become a `mcp_servers` row and must never make Nitro call its own `127.0.0.1` as though that were the user's laptop.

A browser-local port preference may be persisted device-locally. Command-generation preferences such as **Allow terminal network access** are presentation/setup inputs only and must not be shown as authoritative runtime state after a relay is already running.

### AD-003 — Verify remote MCP before persistence

The Add MCP flow should be:

```text
Choose connection → Configure → Scan tools → Review → Add MCP
```

A draft scan endpoint verifies the unsaved configuration through the same client/security path used for stored connections. The final save revalidates server-side and persists only verified server-owned status/tool facts; the browser never submits a trusted tool catalog.

### AD-004 — Do not expose unsupported transport choices

New remote connection UX offers only transports the runtime can actually use (`http` and `sse`). Existing stored `stdio` rows remain visible as unsupported/legacy so the user can remove them, but they stay fail-closed and cannot be presented as healthy.

No server-side child-process execution is introduced as part of this UX refactor.

### AD-005 — Connection truth is server/runtime owned

For remote servers, `status` and discovered `tools` are consequences of server-side verification, not client-editable fields. Updating endpoint/transport invalidates previous discovery and requires revalidation. Metadata-only changes may avoid unnecessary network work.

For the local relay, connection state remains live browser state from `useRelayAgent()` and is not copied into Postgres as durable truth.

### AD-006 — Settings is configuration, not a terminal emulator

The old Local Terminal page's shell runner/history is removed rather than moved into MCP settings. The normal agent/chat execution path remains intact. Connection troubleshooting is limited to bounded connection/discovery feedback.

### AD-007 — UX follows the existing design system

Use Nuxt UI components and semantic theme tokens only. The page should feel like a connection manager, not a dense admin form:

- clear page header + one primary **Add MCP** action;
- concise connection cards with name, kind/transport, status, endpoint, and tool count;
- a menu or explicit manage action instead of a row of ambiguous icon buttons;
- one focused create/manage overlay with progressive disclosure;
- local relay setup expressed as three understandable steps: install, start, verify;
- useful empty/error states with next actions;
- responsive stacking and sticky/visible primary actions on small screens;
- visible keyboard focus and accessible labels;
- primary/cyan reserved for active/connected/focused states per repository convention.

Before implementation, verify the exact installed Nuxt UI component APIs from the project-scoped Nuxt UI MCP/docs rather than guessing props/slots.

### AD-008 — Preserve the stable chat execution boundary

`native.local_terminal` may remain the model-facing/native capability identity during this plan. The goal is to unify management UX, not to rewrite the mature local execution path into generic remote MCP plumbing.

Agent Mode continues to require both the terminal capability toggle and a live local relay. The tool picker should direct users to the unified MCP settings surface when setup is required.

### AD-009 — Web-only validation unless the contract truly crosses stacks

The planned source changes are Nuxt/Vue/TypeScript/server-web changes. Do not compile/test Rust merely because the UI describes the Rust relay. Rust validation becomes required only if implementation unexpectedly changes a shared relay contract or native source.

## Phase overview

| Phase | Goal | Depends on | Exit criterion |
| --- | --- | --- | --- |
| PHASE-01 | Harden MCP connection-management contracts | none | draft scanning and server-owned verification rules are defined/testable without UI coupling |
| PHASE-02 | Build the unified MCP connection UX | PHASE-01 | Settings → MCP manages local + remote connections through a polished responsive overlay/card system |
| PHASE-03 | Migrate local relay setup and remove Local Terminal page | PHASE-02 | no Local Terminal tab/console remains; setup and legacy navigation live under MCP safely |
| PHASE-04 | Reconcile chat/tool-picker behavior | PHASE-03 | Agent Mode/tool selection still work and all setup links target MCP settings |
| PHASE-05 | Automated + browser acceptance and repository closeout | PHASE-01..04 | web gates, build, responsive/live checks, docs/plan truth, and PR delivery are complete |

---

## PHASE-01 — Harden MCP connection-management contracts

**Goal:** make remote MCP discovery and persistence safe enough for a verify-before-save creation UX.

### TASK-001 — Define a narrow remote MCP configuration contract

**Outcome:** create/scan/update paths share an explicit supported remote configuration model instead of accepting arbitrary transport/status/tool input.

**Files:**
- Modify: `shared/types/chat.ts`
- Modify: `server/application/mcp.ts`
- Modify: `server/api/mcp-servers/index.post.ts`
- Modify: `server/api/mcp-servers/[id].put.ts`
- Test: `test/unit/mcp-server-management.test.ts`

**Steps:**
- [ ] Keep compatibility for existing stored `McpTransport` values if needed, but define the user-creatable remote subset as `http | sse`.
- [ ] Validate trimmed name/description and bounded URL/transport input at the API boundary.
- [ ] Remove browser authority to set `status` or `tools` in the update route.
- [ ] Ensure endpoint/transport changes invalidate stale discovery and require re-verification.
- [ ] Preserve user ownership checks for every row mutation.
- [ ] Preserve generic client-facing errors; never echo submitted URL/name or raw SDK/network errors on failure.

**Validation:**
- `pnpm test:web` → feature test proves unsupported transport cannot be newly created, client-supplied status/tools are rejected/ignored by the owning API contract, and mutable fields stay bounded.

**Commit boundary:** `refactor(mcp): tighten server configuration authority`

### TASK-002 — Add draft Scan Tools discovery without persistence

**Outcome:** an unsaved remote MCP configuration can be safely tested and its tool catalog previewed before the user creates a row.

**Files:**
- Create: `server/api/mcp-servers/scan.post.ts`
- Modify: `server/application/mcp.ts`
- Modify: `server/infrastructure/mcp/test-server.ts` or replace with a cohesive server-management module if responsibility becomes clearer
- Modify: `server/infrastructure/mcp/client.ts`
- Modify: `server/infrastructure/composition/application.ts`
- Test: `test/unit/mcp-server-management.test.ts`
- Test/extend when relevant: `test/unit/mcp-error-confidentiality.test.mjs`

**Steps:**
- [ ] Extract one bounded discovery operation that accepts a narrow infrastructure-owned MCP config rather than requiring a Drizzle row purely to connect.
- [ ] Keep the exact current SSRF-safe HTTP/SSE client path and first-party remote owner/token rules.
- [ ] Make draft scan return only safe presentation facts: transport, discovered tool names/descriptions/annotations, and count; never credentials/internal diagnostics.
- [ ] Ensure draft scan performs no DB insert/update.
- [ ] Make final create re-run authoritative server-side discovery and persist `connected + tools` only after successful validation; a failed final verification must not leave an orphaned healthy-looking row.
- [ ] Reuse the same discovery path for **Recheck** on existing rows.

**Validation:**
- `pnpm test:web` → scan is non-persistent, unsafe/unsupported targets fail safely, successful discovery returns bounded tools, and create persists only server-derived status/tools.

**Commit boundary:** `feat(mcp): scan tools before saving connections`

### TASK-003 — Make edit/recheck semantics truthful

**Outcome:** connection cards can safely manage metadata and endpoint changes without stale tool/status claims.

**Files:**
- Modify: `app/composables/useMcpServers.ts`
- Modify: `server/infrastructure/database/mcp-servers.ts`
- Modify: `server/api/mcp-servers/[id].put.ts`
- Test: `test/unit/mcp-server-management.test.ts`

**Steps:**
- [ ] Add composable methods for draft scan and verified create/update rather than forcing the page to sequence low-level requests itself.
- [ ] Keep enable/disable as a lightweight user preference.
- [ ] Recheck uses server-owned discovery and updates tools/status atomically from the result.
- [ ] Endpoint or transport edits cannot retain a previous server's tool list after a failed validation.
- [ ] Existing unsupported `stdio` rows stay visible, disabled/fail-closed, and removable rather than silently converted.

**Validation:**
- `pnpm test:web` → verified update cannot preserve stale discovery across endpoint changes; legacy unsupported rows remain fail-closed.

**Commit boundary:** `refactor(mcp): centralize connection lifecycle actions`

**Phase exit criteria:**
- [ ] Remote MCP configuration can be scanned before save.
- [ ] New creation only offers runtime-supported transports.
- [ ] Browser cannot author connection status/tool catalogs.
- [ ] First-party private credentials and SSRF protections are unchanged.

---

## PHASE-02 — Build the unified MCP connection UX

**Goal:** turn `/settings/mcp` into one calm, readable connection manager for first-party local relay and remote MCP servers.

### TASK-004 — Reshape the MCP page into a thin composition surface

**Outcome:** the 325-line page is decomposed by product responsibility without one-file-per-function fragmentation.

**Files:**
- Modify: `app/pages/settings/mcp.vue`
- Create/Modify: focused files under `app/components/settings/` such as:
  - `SettingsMcpConnectionCard.vue`
  - `SettingsMcpConnectionDialog.vue`
  - `SettingsLocalRelaySetup.vue`
- Test: `test/unit/mcp-settings-ux.test.ts`

**Steps:**
- [ ] Keep the page responsible for page-level composition only.
- [ ] Present a header with **MCP connections**, concise explanatory copy, and one primary **Add MCP** action.
- [ ] Render the built-in local relay first, then user-created remote connections in the same card vocabulary without pretending they share persistence.
- [ ] Cards show only decision-useful information: display name, local/HTTP/SSE kind, current/last-known status, endpoint/port, discovered tool count, and a clear manage/recheck action.
- [ ] Replace the current row of unlabeled icon actions with an accessible explicit action/menu pattern.
- [ ] Keep tool lists collapsible/secondary; do not let large catalogs dominate the settings page.
- [ ] Use semantic color tokens and reserve primary color for connected/active/focused states.

**Validation:**
- `pnpm lint:web`
- `pnpm typecheck:web`
- source/behavior test confirms the page no longer owns the complete form/card implementation and preserves accessible action labels.

**Commit boundary:** `refactor(settings): unify mcp connection presentation`

### TASK-005 — Implement the external MCP client-style Add MCP flow

**Outcome:** remote connection creation is a focused configure/scan/review/create flow rather than create-first/test-later.

**Files:**
- Modify: `app/components/settings/SettingsMcpConnectionDialog.vue`
- Modify: `app/composables/useMcpServers.ts`
- Test: `test/unit/mcp-settings-ux.test.ts`
- Test: `test/unit/mcp-server-management.test.ts`

**Steps:**
- [ ] Start the overlay with a simple connection-kind choice: **Local relay** or **Remote MCP server**.
- [ ] Remote form uses plain product language: Name, Description (optional), Transport, URL.
- [ ] Do not render `stdio` or arbitrary command input for new connections.
- [ ] Provide a distinct **Scan tools** action before **Add MCP**.
- [ ] Show scan progress, bounded failure guidance, and a compact discovered-tools preview.
- [ ] Disable final Add until the current draft has a successful scan; changing endpoint/transport invalidates the preview.
- [ ] Revalidate on final save so stale client preview is never treated as authority.
- [ ] Reset draft state when the overlay closes/reopens; never leak one server's discovered tools into another draft.
- [ ] Use the same overlay structure for edit/manage where practical so terminology and action placement stay consistent.

**Validation:**
- `pnpm test:web` → state transitions cover pristine → scanning → scan error → scanned → edited-invalidated → created.
- keyboard/form labels and error relationships are inspectable in generated/component markup.

**Commit boundary:** `feat(settings): add verified mcp creation flow`

### TASK-006 — Polish responsive and empty/error states

**Outcome:** MCP settings remains readable at desktop and phone widths and guides the user through failure instead of exposing implementation noise.

**Files:**
- Modify: MCP settings components from TASK-004/TASK-005

**Steps:**
- [ ] Desktop cards align status/actions without dense horizontal packing.
- [ ] Mobile cards stack metadata and actions without horizontal overflow.
- [ ] Overlay body scrolls independently when local setup/tools are long, with final actions always reachable.
- [ ] Empty remote state explains that the local relay is built in and offers **Add MCP** for additional servers.
- [ ] Connection errors explain the next action (check URL/relay, recheck) without raw SDK/provider diagnostics.
- [ ] Destructive remove action requires an intentional confirmation pattern appropriate to Nuxt UI.
- [ ] Respect reduced-motion; use only subtle state transitions if any.

**Validation:**
- browser review at representative desktop and narrow mobile widths after a production build.

**Commit boundary:** `style(settings): polish mcp connection management`

**Phase exit criteria:**
- [ ] `/settings/mcp` is visually coherent and contains local + remote connection management.
- [ ] Add MCP uses verify-before-save.
- [ ] No unsupported transport is offered.
- [ ] Desktop/mobile and keyboard states are usable.

---

## PHASE-03 — Migrate local relay setup and remove Local Terminal page

**Goal:** retain useful local-relay onboarding under MCP while deleting the separate terminal settings product surface and dead console code.

### TASK-007 — Move local relay setup into the unified manage overlay

**Outcome:** users can install/start/check the first-party local relay entirely from Settings → MCP.

**Files:**
- Modify: `app/components/settings/SettingsLocalRelaySetup.vue`
- Modify: `app/composables/useRelayAgent.ts`
- Optionally create when it keeps ownership clearer: `app/composables/useLocalRelaySettings.ts`
- Test: `test/unit/local-relay-settings.test.ts`

**Steps:**
- [ ] Present three explicit setup steps: **Install relay**, **Start relay**, **Verify connection**.
- [ ] Preserve Linux x86_64 download target and non-root/Bubblewrap requirements from current UI.
- [ ] Preserve foreground/background command examples using the actual current page origin and configured/default local port.
- [ ] Keep **Allow terminal network access** as an opt-in that changes the generated launch command only; explain that changing the toggle does not mutate an already-running process.
- [ ] Keep the default port `47821`; if custom port configuration is exposed, persist it browser/device-locally rather than in user-scoped Postgres.
- [ ] Upgrade connection verification from a vague visual check to the strongest bounded browser-local relay handshake already supported without changing the Rust protocol; at minimum keep existing health behavior and verify MCP compatibility where current browser relay APIs allow it.
- [ ] Show connected/disconnected/checking state consistently with the connection card.

**Validation:**
- `pnpm test:web` → command generation preserves origin/port/network flag semantics and local settings do not create remote MCP database requests.

**Commit boundary:** `feat(settings): move local relay setup under mcp`

### TASK-008 — Delete the Settings terminal console and dead job helpers

**Outcome:** Settings no longer contains an arbitrary command runner and `useRelayAgent()` keeps only execution APIs still owned by chat/agent flows.

**Files:**
- Delete: `app/pages/settings/local-terminal.vue`
- Modify: `app/composables/useRelayAgent.ts`
- Test: `test/unit/local-relay-settings.test.ts`

**Steps:**
- [ ] Remove the terminal history/input/run/cancel UI rather than embedding it into the new MCP page.
- [ ] Confirm `startJob()`, `getJob()`, `cancelJob()`, `fallbackJobCall()`, and Settings-only result types have no remaining callers before deleting them.
- [ ] Preserve `exec()`, task-aware MCP execution, session lifecycle calls, connection state, and all APIs used by chat/agent execution.
- [ ] Do not touch Rust fallback/task behavior unless a real surviving client caller requires it.

**Validation:**
- repository search shows no dead `/settings/local-terminal` component import or Settings job-helper caller.
- `pnpm typecheck:web` and `pnpm test:web` pass.

**Commit boundary:** `refactor(settings): remove local terminal console`

### TASK-009 — Remove the tab and preserve old deep links

**Outcome:** Settings navigation contains MCP only, while old bookmarks do not strand users on a 404.

**Files:**
- Modify: `app/pages/settings.vue`
- Modify the verified Nuxt-native routing owner for a redirect, likely `nuxt.config.ts` route rules or an equivalent current Nuxt mechanism after checking the installed Nuxt routing API
- Test: `test/unit/mcp-settings-ux.test.ts`

**Steps:**
- [ ] Remove `Local Terminal` from the settings navigation array.
- [ ] Redirect `/settings/local-terminal` to `/settings/mcp` using the Nuxt-native mechanism supported by the installed version.
- [ ] Do not keep a duplicate settings page solely for compatibility.
- [ ] Update any user-facing copy that still tells users to open Local Terminal settings.

**Validation:**
- built application navigation contains no Local Terminal tab.
- legacy route resolves to MCP settings rather than 404.

**Commit boundary:** `refactor(settings): retire local terminal route`

**Phase exit criteria:**
- [ ] No Local Terminal tab or settings console remains.
- [ ] Local relay onboarding is complete under MCP.
- [ ] Old route redirects cleanly.
- [ ] Normal chat execution APIs remain intact.

---

## PHASE-04 — Reconcile chat and tool-picker behavior

**Goal:** make the rest of the product speak the same connection-management language without changing execution authority.

### TASK-010 — Point terminal/Agent Mode setup to MCP settings

**Outcome:** disconnected users are directed to the new unified connection manager everywhere.

**Files:**
- Modify: `app/components/chat/ChatToolPicker.vue`
- Review/modify only if needed: `app/components/chat/ChatConfigControls.vue`
- Review: `shared/utils/native-tools.ts`
- Test: `test/unit/mcp-settings-ux.test.ts`

**Steps:**
- [ ] Replace **Terminal settings** deep link with **Manage MCP** / **MCP settings** targeting `/settings/mcp`.
- [ ] Keep the terminal capability toggle and Agent Mode gating based on live `useRelayAgent()` state.
- [ ] Ensure disconnected local relay presentation uses the same words/status concepts as the MCP settings card.
- [ ] Avoid duplicating setup instructions inside the tool picker; it should link to management rather than become a second settings surface.
- [ ] Keep remote MCP tool enablement behavior unchanged unless required to support the new verified connection lifecycle.

**Validation:**
- `pnpm test:web` → Agent Mode remains unavailable when relay is disconnected/terminal capability is off, and setup link targets MCP settings.

**Commit boundary:** `refactor(chat): route tool setup through mcp settings`

### TASK-011 — Review native/local naming for user clarity

**Outcome:** implementation identity can stay stable while visible copy consistently describes the capability as a local relay/MCP connection rather than a separate Local Terminal product.

**Files:**
- Review/modify: `shared/utils/native-tools.ts`
- Modify relevant MCP settings/chat copy only where necessary

**Steps:**
- [ ] Preserve `NATIVE_LOCAL_TERMINAL_TOOL_ID` if renaming it would create unnecessary persisted conversation migration risk.
- [ ] Improve only user-visible label/description where needed, e.g. **Local relay** / **Terminal via local relay**.
- [ ] Do not rewrite stable tool IDs solely for cosmetic consistency.

**Validation:**
- existing persisted tool IDs remain valid and user-visible terminology is consistent.

**Commit boundary:** fold into TASK-010 unless a separate identity-safe change is independently reviewable.

**Phase exit criteria:**
- [ ] No chat UI links to the removed route.
- [ ] Agent Mode behavior is preserved.
- [ ] User-visible terminology matches the new connection model.

---

## PHASE-05 — Acceptance, documentation, and repository closeout

**Goal:** prove the refactor works as a product flow, not only as compiled source, and close it according to repository policy.

### TASK-012 — Add focused feature tests

**Outcome:** durable tests cover the new connection-management behavior without plan-numbered scripts.

**Files:**
- Create: `test/unit/mcp-server-management.test.ts`
- Create: `test/unit/mcp-settings-ux.test.ts`
- Create or combine if cohesion is better: `test/unit/local-relay-settings.test.ts`
- Extend: `test/unit/mcp-error-confidentiality.test.mjs` only for genuinely new error surfaces

**Required coverage:**
- remote draft scan does not persist;
- new creation allows supported transports only;
- server status/tools cannot be authored by the browser;
- successful scan produces bounded discovered-tool presentation;
- endpoint change invalidates stale scan/tool state;
- unsupported legacy `stdio` remains fail-closed/removable;
- local relay management does not create a Postgres MCP row;
- generated local relay commands preserve origin/default-or-configured port and opt-in network flag;
- settings navigation has no Local Terminal tab;
- legacy route redirect is configured;
- chat tool picker points setup to `/settings/mcp`;
- generic errors do not include raw submitted endpoint/SDK diagnostics.

**Validation:**
- `pnpm test:web`

**Commit boundary:** tests should land with the behavior they prove; do not create a plan-only verification commit/script.

### TASK-013 — Production-build and browser UX verification

**Outcome:** the actual built Nuxt app proves the settings flow visually and behaviorally.

**Steps:**
- [ ] Run `pnpm build`.
- [ ] Run/restart `pnpm preview` rather than trusting a stale dev process after route/component moves.
- [ ] Verify desktop Settings navigation and MCP page hierarchy.
- [ ] Verify narrow/mobile layout for cards, overlay, tool preview, and action footer.
- [ ] Verify keyboard focus order and labels through add/manage/remove/recheck flows.
- [ ] Verify remote scan failure does not create a row; verify a safe available MCP fixture if one is available.
- [ ] Verify local relay disconnected onboarding.
- [ ] If the local relay is live, verify connected status and Agent Mode availability end-to-end; if the fixture is unavailable, record that live acceptance as unproven rather than fabricating it.
- [ ] Verify `/settings/local-terminal` redirects to `/settings/mcp`.
- [ ] Inspect browser console/network for unexpected errors or duplicate requests.

**Validation:**
- `pnpm build` → success
- browser verification → no layout overflow, broken route, stale state, or raw diagnostic leakage

**Commit boundary:** no special acceptance script; fixes discovered here belong in the owning feature commit.

### TASK-014 — Run repository gates and reconcile durable guidance

**Outcome:** Plan 051 closes with truthful docs/status and repository-required local quality evidence.

**Files:**
- Modify: `.agents/plans/051-unified-mcp-settings-and-local-relay-ux.md`
- Modify: `.agents/memories/README.md` only if implementation establishes a durable architecture decision/trap not already obvious from code/plan
- Modify user/operator docs only where they currently instruct users to use the separate Local Terminal settings page

**Steps:**
- [ ] Search docs/source for stale `/settings/local-terminal`, **Local Terminal settings**, unsupported `stdio` creation guidance, and create-first/test-later assumptions.
- [ ] Update only durable guidance that became false.
- [ ] Run `pnpm lint:web`.
- [ ] Run `pnpm typecheck:web`.
- [ ] Run `pnpm test:web`.
- [ ] Run `pnpm guardrail` and confirm it scopes to the touched web/docs stack unless implementation truly changed Rust/shared native contract.
- [ ] Review `git diff` and `git status`; leave unrelated `workspaces/` content untouched.
- [ ] Update plan status/checklists with exact evidence; do not claim unavailable live fixtures.
- [ ] Follow short-lived branch → focused commit(s) → push → PR into `main`; do not bypass hooks and do not merge without the authorization required by repository policy.

**Validation:**
- `pnpm guardrail` → PASS
- final tracked diff contains only Plan-051-owned changes

**Commit boundary:** `docs(agents): close unified mcp settings plan` only if a separate closeout-doc commit is useful; otherwise include truthful plan/doc updates with the final implementation commit.

**Phase exit criteria:**
- [ ] All applicable web gates and guardrail pass.
- [ ] Production build/browser review passes.
- [ ] Plan/docs/memory match shipped behavior.
- [ ] No unrelated files are staged.
- [ ] PR contains exact local verification evidence.

## Risks and rollback

- **Risk — localhost is accidentally treated as a remote server:** Nitro would connect to its own loopback, not the user's laptop, and could weaken the intended security model. **Mitigation:** keep local relay browser-owned and explicitly separate from Postgres remote MCP rows. **Rollback:** revert local connection adapter/UI without changing relay runtime.
- **Risk — draft scanning becomes a new SSRF surface:** arbitrary user URLs are probed before persistence. **Mitigation:** reuse the exact existing SSRF-safe MCP client and owner-bound first-party credential rules; draft state must not bypass them. **Rollback:** disable draft scan endpoint and restore stored-row test flow while retaining the unified UI shell.
- **Risk — stale discovered tools after editing endpoint:** tools from server A could appear under server B. **Mitigation:** invalidate scan state on connection-defining edits and make server-side final validation authoritative. **Rollback:** require explicit recheck before enabling edited rows.
- **Risk — local relay command toggle is mistaken for live runtime state:** UI cannot mutate a process that is already running. **Mitigation:** label network toggle as launch-command generation only. **Rollback:** show static documented flag instead of a toggle.
- **Risk — deleting the settings console removes a troubleshooting path:** some manual testing convenience disappears. **Mitigation:** keep bounded connection/recheck diagnostics; normal agent execution remains in chat. **Rollback:** restore a dedicated developer-only diagnostic in a future plan only if a real product need is proven, not by re-expanding Settings into a shell.
- **Risk — cosmetic ID rename breaks persisted conversations:** existing enabled tool IDs use `native.local_terminal`. **Mitigation:** preserve stable internal ID unless a migration is intentionally designed. **Rollback:** revert visible naming only; no data migration required.
- **Risk — component extraction becomes wrapper spam:** splitting the large pages mechanically would violate repository maintainability intent. **Mitigation:** create only cohesive settings components with distinct responsibility (card, dialog, local setup). **Rollback:** fold pass-through wrappers back into the owning component.
- **Risk — docs/plan refactor accidentally triggers unrelated Rust work:** UI describes a Rust process but does not change its contract. **Mitigation:** keep validation web-scoped and expand to Rust only on a verified shared-contract change.

## Final acceptance criteria

- [ ] Settings tabs contain General, Models, MCP, API Keys, Account; no Local Terminal tab remains.
- [ ] `/settings/mcp` is the only user-facing management page for the built-in local relay and remote MCP connections.
- [ ] Old `/settings/local-terminal` deep links redirect to MCP settings.
- [ ] Local relay stays browser/device-owned and never becomes a generic server-side localhost MCP row.
- [ ] Remote Add MCP flow is configure → Scan tools → review → Add.
- [ ] Final remote persistence is server-verified and browser cannot author `status` or `tools`.
- [ ] New UI does not offer server-side `stdio` execution.
- [ ] Existing unsupported `stdio` rows remain fail-closed and removable.
- [ ] Local relay setup preserves install/start guidance, origin, port behavior, and terminal-network opt-in.
- [ ] Settings contains no arbitrary shell command runner/history.
- [ ] Agent Mode and terminal tool selection still depend on the real local relay connection.
- [ ] Chat setup links route to `/settings/mcp`.
- [ ] UI is responsive, accessible, semantically themed, and has useful empty/error/loading states.
- [ ] `pnpm lint:web`, `pnpm typecheck:web`, `pnpm test:web`, `pnpm build`, and `pnpm guardrail` pass.
- [ ] Live local/remote checks are recorded truthfully where fixtures are available; unavailable external acceptance is not claimed.
- [ ] Repository agent/docs guidance is reconciled and no stale Local Terminal settings instructions remain.
- [ ] Changes are delivered through the repository-required short-lived branch and PR workflow without bypassing hooks.

## Execution handoff

Execute phases in order. PHASE-01 is the security/data-contract prerequisite for the new create flow; PHASE-02 may begin once the draft-scan contract is stable. PHASE-03 and PHASE-04 should remain on the same Plan-051 branch because they are one UX migration and share the local relay state boundary. Test work should land alongside each owning behavior rather than being deferred into a plan-numbered acceptance layer.

Before implementation starts, re-check `main`, current worktrees, and any concurrent changes to `app/pages/settings/*`, `app/components/chat/*`, `app/composables/useRelayAgent.ts`, MCP server routes, or MCP infrastructure. Preserve unrelated untracked `workspaces/` content and do not reset it.
