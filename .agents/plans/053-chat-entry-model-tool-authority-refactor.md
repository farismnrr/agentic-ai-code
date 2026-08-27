# Plan 053 — Chat Entry Model, Tool Surface, and Authority Refactor

**Status:** CLOSED / VERIFIED
**Created:** 2026-08-27

## Goal

Make the new-chat experience predictable and scalable while enforcing tool authority on the server:

- Resolve the initial model from the most recently used model in the active workspace, then the configured global default, otherwise require an explicit model choice with a deliberate empty state.
- Remove MCP tool enumeration and per-tool selection from the chat composer. MCP connection/tool configuration is owned by Settings.
- Make Chat and Agent availability depend on terminal capability, not on MCP tool count/selection.
- Chat remains available when terminal capability is absent. With terminal capability, Chat is read-only and Agent is read-write-capable subject to permission mode.
- Enforce read-only/read-write authority in backend composition; UI state is not a security boundary.

## Product semantics

### Model resolution

For a new conversation in workspace W:

1. newest conversation in W with a currently valid model ID;
2. `settings.defaultModelId` when currently valid;
3. no model selected.

When no model is selected, show an explicit `Choose a model` state, explain what is required, and block submission until the user selects one.

### Tool configuration surface

- Do not render MCP server tools in the new-chat or existing-chat composer.
- Settings → MCP is the single product surface for MCP connection/enabled state/tool inventory.
- Preserve persisted `enabledToolIds` for compatibility during this refactor, but do not use a composer picker as the source of truth for new conversations.
- Resolve effective connected MCP capabilities server-side.

### Chat and Agent modes

- Chat is always available.
- If the first-party terminal relay capability is unavailable/disabled/disconnected, Chat is plain model chat with no terminal/coding tool access and Agent is unavailable.
- If terminal capability is available, Chat may use terminal/coding capabilities only under read-only authority.
- Agent is available only while terminal capability is available and may use read-write capabilities according to permission mode.
- Permission mode is relevant to Agent; hide it in Chat.
- Persisted Agent conversations must fail closed to Chat/read-only behavior if terminal capability is no longer available.

## Implementation phases

### A. New-chat model resolver

- Add a reusable resolver derived from sidebar conversation metadata and the active workspace.
- Re-resolve when workspace changes without overriding an explicit user selection unnecessarily.
- Add accessible empty-model UX and submission guard.

### B. Composer cleanup

- Remove `ChatToolPicker` from `ChatConfigControls` and all new-chat/existing-chat tool-count UI.
- Keep the composer focused on workspace, mode, permission when relevant, model, and reasoning effort.

### C. Terminal capability state

- Introduce a single server/client-facing terminal-capability semantic sourced from the configured first-party remote relay rather than arbitrary remote MCP tools.
- Use the same semantic to gate Agent presentation and backend effective mode.

### D. Backend authority refactor

- Resolve effective mode/authority at execution time.
- Chat + terminal: allow only read effects (`workspace_read`, `git_read`) and read-safe tools.
- Agent + terminal: allow bounded read/write/process/network/external effects according to permission mode.
- No terminal: no coding/terminal tool composition, regardless of persisted Agent state.

### E. MCP capability resolution

- Stop relying on composer-selected MCP IDs for new conversations.
- Resolve enabled/connected MCP tools on the server from account-scoped MCP settings, while preserving compatibility with existing persisted records.
- Apply effect/approval filtering before exposing tools to the model.

### F. Regression coverage

Cover at minimum:

1. workspace-last-model wins;
2. global default fallback;
3. explicit empty-model UX and submit guard;
4. workspace switch model resolution;
5. no MCP tool enumeration in composer;
6. MCP tool volume does not affect composer size or Agent availability;
7. no terminal → Chat only and no coding tools;
8. terminal available → Chat read-only and Agent available;
9. Chat cannot receive mutating effects/tools;
10. Agent authority follows permission mode;
11. terminal loss makes persisted Agent fail closed;
12. server-side MCP settings remain the canonical connection/tool source.

## UX revision: read-only actions do not prompt

- Read-only capabilities auto-approve in manual mode.
- Approval UI is reserved for state-changing effects (workspace writes/deletes, process execution, network writes, external mutations, privileged bridges).
- Reviewed direct terminal reads (`cat`, `head`, `tail`, `ls`, `pwd`, `rg`, `grep`) narrow to `workspace_read` instead of inheriting broad terminal mutation effects.
- Malformed input and protected credential paths remain fail-closed, including direct reads such as `cat .env`.
- Unknown external MCP tools remain conservative unless their resolved effects are genuinely read-only.

## Constraints

- Keep OAuth tokens, relay bearer tokens, client secrets, authorization codes, PKCE values, and other credentials server-only and out of telemetry/logs.
- Do not weaken SSRF, ownership, approval, or effect filtering boundaries.
- Do not drop compatibility DB columns in this refactor unless required by correctness.
- Do not change/restart the separately managed relay service unless explicitly required and approved.
- Use repository MCP Git tools for Git operations.

## Definition of done

- Unit/feature regression tests pass.
- Web typecheck and lint pass.
- Production build passes.
- Runtime UI behavior is verified after deployment.
- Plan status is updated with concrete validation results.

## Closure evidence — 2026-08-27

- New-chat model resolution, explicit empty-model UX, and composer MCP-tool removal implemented.
- Chat/Agent availability now follows runtime terminal capability instead of composer-selected MCP tool volume.
- Chat read-only authority is enforced server-side; mutating capabilities remain Agent-only and permission-controlled.
- Read-only capability calls auto-approve while write/delete/external mutation and opaque execution retain approval or fail-closed handling.
- MCP Settings is the canonical execution source; stale legacy first-party rows without viable OAuth/runtime credentials are excluded.
- OAuth-backed relay reconnect supports modern MCP fallback, refresh-token retry, and `offline_access` for durable connector sessions.
- Browser E2E reproduced the original no-tools failure and verified the repaired capability/OAuth flow against the live deployment.
- `pnpm test:web`: 28/28 pass.
- `pnpm typecheck:web`: pass.
- Web + Rust lint/clippy validation: pass.
- Production Nuxt build: pass with build-only generated session secret for prerender validation.
- Live runtime health: HTTP 200 after deployment.
