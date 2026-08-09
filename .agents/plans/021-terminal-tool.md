# 021 — Terminal (shell exec) tool: read-only in chat mode, full access in agent mode

## Context

The user wants an LLM-callable terminal tool — this is a coding assistant, and reading/exploring a repo from the shell is core to that. Two modes already exist per conversation (`Conversation.mode: 'chat' | 'agent'`, `shared/types/chat.ts:66`), each with its own, currently-separate tool pipeline:

- **Chat mode** (`server/utils/langgraph-chat.ts`) runs LangChain's `createAgent` with a fixed, non-configurable tool array, `langgraphTools` (`server/utils/langgraph-tools.ts`) — currently `curl` + `searxng_search`, both built as `packages/<name>-tool` local workspace packages (plan 020).
- **Agent mode** (`server/api/chat.post.ts:47`) runs the `ai@7` SDK's `streamText` with a `ToolSet` built entirely from user-toggled MCP servers via `buildMcpTools()` (`server/utils/mcp-tools.ts`) — no native (non-MCP) tool exists in this path today.

The ask: chat mode gets the terminal tool **read-only** (safe, always-on, like curl/searxng); agent mode gets it **full access**, but only when the user explicitly turns it on, mirroring the existing per-conversation MCP tool toggle + approval flow. Confirmed with the user: agent-mode full access still runs inside its conversation's workspace directory, never outside it.

Researched and adopted from how other coding assistants (Claude Code, OpenHands, Aider) built this, and from a real disclosed vulnerability class in Claude Code's own Bash tool (CWE-78 command injection via shell-interpolated strings):

- **Never invoke a shell.** Accept `command` + `args[]` as separate fields, execute via `execa(command, args, { shell: false, ... })` (the LLM-facing schema literally cannot contain shell metacharacters that matter, because nothing parses them).
- Use `execa` over raw `child_process` — no shell by default, real `timeout` with process-tree kill, structured results, cross-platform argv handling.
- Read-only mode is an **allowlist of exact binaries with per-binary argv validation**, not a blacklist — blacklists are the class of bug that keeps recurring (`git commit` vs `git status`, `find -delete`, `sed -i`, output redirection).
- Regardless of mode: execution timeout, output truncation, and a restricted `cwd` + minimal `env` passthrough (never the full `process.env`).

**A jail boundary already exists in this codebase and should be reused, not reinvented**: `server/utils/fs-browse.ts`'s `resolveWorkspacePath()` is the exact fail-closed, symlink-aware traversal guard (`isWithinRoot`, re-checked against `fs.realpath`) that `createWorkspace`/`updateWorkspace` (`server/utils/workspaces.ts`) already run every workspace path through before storing it. A conversation's `workspaceId` → `workspaces.path` is therefore already a validated, jailed absolute directory under `NUXT_WORKSPACES_ROOT`. The terminal tool's `cwd` is this workspace path — for both modes — not `process.cwd()`. This is the same "don't fork the guard, reuse the one implementation" principle already established for `assertSafeUrl` (`.agents/memories/012-mcp-outbound-tool-loop.md`).

## Decisions

- **New package `packages/terminal-tool/`**, following the exact `packages/curl-tool` layout (`package.json`, `src/index.ts`, `bin/cli.mjs`, `SKILL.md`), added to the pnpm workspace and consumed via `workspace:*`, per plan 020's established convention.
- **One shared zod schema, one core executor, two thin factory wrappers** — the package exports:
  - `terminalToolSchema` — `z.object({ command: z.string(), args: z.array(z.string()).optional() })`.
  - `runTerminalCommand({ command, args, cwd, assertSafeCommand })` — the actual `execa` call: validates via the injected `assertSafeCommand(command, args)` (fail-closed — throwing blocks execution), runs with `shell: false`, `cwd`, a minimal `env` (`PATH`, `HOME`, `LANG` only — never the parent's full env), a `timeout` (30s, `killSignal: 'SIGKILL'`), and truncates stdout/stderr independently (20,000 chars each, mirroring `curl-tool`'s existing 10,000-char truncation). Returns `Exit: <code>\nStdout: ...\nStderr: ...`.
  - `createTerminalTool({ assertSafeCommand, cwd })` — LangChain `tool()` wrapper (chat mode), same shape as `createCurlTool`.
  - `createTerminalAiTool({ assertSafeCommand, cwd })` — `ai` SDK `tool()` wrapper (agent mode), reusing the same `terminalToolSchema` as its `inputSchema` and the same `runTerminalCommand` core — this is the DI boundary that keeps the safety logic singular across both frameworks, exactly as `assertSafeUrl` does for `curl-tool`.
  - `bin/cli.mjs` — standalone runner; default behavior blocks (safe-by-default) unless `--no-guard`, matching `curl-tool`'s CLI convention.
- **New app-owned guard, `server/utils/exec-guard.ts`** (sibling to `ssrf-guard.ts`), exporting one function: `assertSafeCommand(command: string, args: string[], mode: 'read-only' | 'full'): Promise<void>`.
  - `mode: 'read-only'` (chat mode, hard-wired, not model-controllable): allowlist of exact binaries with per-binary argv checks — `ls`, `cat`, `pwd`, `echo`, `grep`, `rg` (ripgrep — faster repo-wide search than `grep -r`, auto-excludes `.git`/`node_modules`/build output without needing `--exclude-dir` on every call), `head`, `tail`, `wc`, `stat`, `file`, `tree`, `diff`, `find` (block `-delete`/`-exec`/`-fprintf`), `sed` (only `-n`, block `-i`), `git` (only `status`/`log`/`diff`/`show`/`branch`/`remote -v`; block `commit`/`push`/`reset`/`checkout`/`clean`/`stash drop`/`config --global`/`-C`/`--exec`). Anything not explicitly recognized is rejected — same fail-closed posture as `assertSafeUrl`'s "unrecognized address shape ⇒ disallowed".
  - Together, `tree`/`find` (map structure) → `rg`/`grep -rn` (locate by keyword) → `wc -l` + `sed -n '<range>p'` (read only the relevant line range) cover browsing a large codebase without ever needing to `cat` a whole large file or directory dump.
  - `mode: 'full'` (agent mode, opt-in): no binary allowlist — this is the "full access" the user asked for. The safety net here is the shared `cwd` jail (below) plus the timeout/env/output limits already baked into `runTerminalCommand`, which apply unconditionally regardless of mode.
  - This is the **one** function both pipelines call — chat mode always passes `'read-only'`, agent mode always passes `'full'` — so there is a single place that encodes the command-safety rules, matching the existing `assertSafeUrl` precedent.
- **cwd resolution reuses `resolveWorkspacePath`** — no new path-jailing logic. Both wiring sites resolve `conv.workspaceId → workspaces.path` (already validated at workspace-creation time) and pass that directory as `cwd`. If a conversation has no workspace, the terminal tool is not wired in for that request (fail closed, not "fall back to process cwd").
- **Chat mode wiring is always-on** (matches curl/searxng's existing "fixed, non-configurable" convention) — no new toggle needed, per plan 020's precedent.
- **Agent mode wiring is opt-in, reusing the existing MCP tool enable/approval data model** rather than adding new schema or a new UI pattern:
  - A reserved, non-MCP tool id `'native.terminal'` (new constant, e.g. exported from `shared/utils/native-tools.ts`) can appear in `conv.enabledToolIds` and `conv.approvals` exactly like a real `McpTool['id']` — no DB migration needed, both columns are already just `text[]`/`jsonb`.
  - `server/api/chat.post.ts`, after `buildMcpTools(...)`: if `conv.enabledToolIds.includes('native.terminal')` and the conversation has a workspace, merge `tools['terminal'] = createTerminalAiTool({ assertSafeCommand: (c, a) => assertSafeCommand(c, a, 'full'), cwd: workspacePath })` into `tools`, and set `toolApproval['terminal']` from `conv.approvals['native.terminal']` using the exact same `'always' → 'approved'`, `'never' → 'denied'`, else `'user-approval'` mapping `mcp-tools.ts` already uses. This means the existing `ChatToolApproval.vue` modal gates every terminal call automatically — no new approval UI needed.
  - `app/components/ChatToolPicker.vue` gets one new static entry (not sourced from `useMcpServers()`) for "Terminal (full shell access)", bound to the same `v-model` via the reserved id — reuses the component's existing `isOn`/toggle logic, no new picker mechanism.

## Changes

### `packages/terminal-tool/`
- `package.json` — name `@ai-code/terminal-tool`, private, `type: module`, `bin: { terminal-tool: ./bin/cli.mjs }`, deps: `execa`, `@langchain/core`, `ai`, `zod`.
- `src/index.ts` — `terminalToolSchema`, `runTerminalCommand`, `createTerminalTool`, `createTerminalAiTool` as described above.
- `bin/cli.mjs` — argv-driven runner (`command`, repeated `--arg`, `--cwd`, `--no-guard`), unsafe-by-default guard requiring `--no-guard` to actually run, mirroring `packages/curl-tool/bin/cli.mjs`.
- `SKILL.md` — same frontmatter/structure as `packages/curl-tool/SKILL.md`, documenting the LangChain factory, the `ai` SDK factory, and the CLI.

### App wiring
1. `pnpm-workspace.yaml` / root `package.json`: add `@ai-code/terminal-tool: workspace:*` (glob `packages/*` already covers it from plan 020).
2. `server/utils/exec-guard.ts` (new) — `assertSafeCommand(command, args, mode)` as decided above.
3. `shared/utils/native-tools.ts` (new, small) — export the `NATIVE_TERMINAL_TOOL_ID = 'native.terminal'` constant (and a `NativeTool[]` list shape so it's the one place to extend if more native agent-mode tools show up later).
4. `server/utils/langgraph-tools.ts`: convert the static `langgraphTools` array export into a factory, e.g. `buildLanggraphTools({ workspacePath }: { workspacePath?: string })`, adding `createTerminalTool({ assertSafeCommand: (c, a) => assertSafeCommand(c, a, 'read-only'), cwd: workspacePath })` to the returned array only when `workspacePath` is defined (curl/searxng stay unconditional). This requires `langgraph-chat.ts`'s `runLanggraphChat` to accept and thread through a `workspacePath` param.
5. `server/utils/langgraph-chat.ts`: accept `workspacePath` in `runLanggraphChat(...)`, call `buildLanggraphTools({ workspacePath })` instead of importing the static array.
6. `server/api/chat.post.ts`: look up the conversation's workspace (`workspaces` table by `conv.workspaceId`) once per request; pass its `path` to `runLanggraphChat(...)` for chat mode, and to the new agent-mode terminal-tool merge block described above.
7. `app/components/ChatToolPicker.vue`: add the static "Terminal (full shell access)" checkbox entry using `NATIVE_TERMINAL_TOOL_ID`.

## Out of scope

- Container/VM-level sandboxing (Docker, `unshare -n`, restricted OS user) — the workspace-path jail plus argv-only exec (no shell) is the safety boundary for this deployment; full isolation is a larger infra change, not part of this plan.
- Disabling network access during read-only-mode command execution — no such control exists without containerization; read-only safety here comes entirely from the binary/argv allowlist, not network isolation.
- Any change to the MCP approval mechanism itself, or to `enabledToolIds`/`approvals` schema — the reserved-id approach deliberately reuses both unchanged.
- Bundling `ripgrep` as an npm dependency — `rg` is allowlisted as a system binary the deployment environment must already have installed (same assumption as `git` already being present); if it's missing, `assertSafeCommand` still rejects unknown binaries safely, `rg` just won't be usable until installed. Verify it's present in whatever image/host runs this Nitro server before relying on it.
- Multi-tenant/per-request sandbox process pooling — out of scope, `execa` runs in the Nitro server process itself, same trust boundary as the rest of `server/utils/`.

## Verification

- `pnpm install` — new package resolves as a workspace symlink.
- `pnpm lint`, `pnpm typecheck`, `pnpm build`.
- `npx --package=./packages/terminal-tool terminal-tool ls --cwd . --no-guard` — CLI runs standalone.
- Manual, chat mode: ask the model to run `ls`, `git log`, `cat <file>` — confirm results come back; ask it to attempt something write-like (e.g. `rm`, `git commit`) and confirm `assertSafeCommand` rejects it with a clear error the model can see and report.
- Manual, agent mode: with `native.terminal` NOT in `enabledToolIds`, confirm the model has no terminal tool available at all. Toggle it on via `ChatToolPicker.vue`, confirm the approval modal (`ChatToolApproval.vue`) appears on first call, and that "always allow" persists via `conv.approvals['native.terminal']` and skips the modal on subsequent calls.
- Manual, agent mode, full access: confirm a write command (e.g. creating a file) succeeds inside the conversation's workspace path, and confirm a path-traversal attempt (e.g. `cat ../../etc/passwd` or an absolute path outside the workspace) is rejected — trace that rejection back to `resolveWorkspacePath`'s existing boundary check, not a new one.
- Confirm `server/utils/exec-guard.ts` has exactly one `assertSafeCommand` implementation shared by both `langgraph-tools.ts` (mode `'read-only'`) and `chat.post.ts` (mode `'full'`) — no forked copy.
