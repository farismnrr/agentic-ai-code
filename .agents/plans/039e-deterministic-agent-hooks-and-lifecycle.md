# Plan 039E — Deterministic Agent Hooks and Lifecycle

**Status:** IMPLEMENTED — FINAL INDEPENDENT VERIFICATION PENDING
**Created:** 2026-08-16  
**Parent:** [Plan 039 — Coding Agent Platform Parity Roadmap](039-coding-agent-platform-parity-roadmap.md)  
**Depends on:** Plan 039D  

## Goal

Add a vendor-neutral, deterministic lifecycle-hook layer for coding-agent workflows so formatting, validation, policy checks, context injection, and stop conditions can run reliably at defined lifecycle points rather than depending on the model to remember them.

## Design principle

Hooks are executable policy/automation, not prompt advice. They must be bounded, observable, and subordinate to Plan-039D hard policy. A hook may preserve or narrow authority; it may never widen a deny, grant secrets, escape the sandbox, or bypass normal approval/security enforcement.

## Initial hook events

Implement only events with clear current value:

- `session_start`
- `pre_tool_use`
- `post_tool_use`
- `tool_error`
- `after_file_change`
- `pre_agent_stop`
- `subagent_stop` (activated after Plan 039F)

Avoid an unbounded generic event bus.

## Handler types

### Required v1

- bounded local command handler using direct argv;
- optional built-in/internal handler for repository-owned deterministic checks.

### Deferred until justified

- arbitrary HTTP callbacks;
- LLM/prompt hooks;
- hook-chained subagents;
- remote webhook automation.

The first implementation should be boring and deterministic.

## Configuration and trust

Use a vendor-neutral repository-owned format under `.agents/` if repository hook configuration is needed. Exact filename/schema should be chosen during implementation after checking current conventions.

Repository hook configuration is executable content and therefore **must not auto-run merely because a cloned repository contains it**.

Required trust model:

- built-in repository hooks shipped with this trusted project may run according to operator/session policy;
- newly encountered project hook configuration requires explicit trust/enablement before execution;
- trust is scoped to canonical repository identity, not only path;
- hook commands must resolve inside reviewed safe PATH/toolchain policy;
- hook working directory is contained and explicit;
- project hooks cannot request sudo, protected credentials, or broaden network/filesystem authority;
- invalid config disables affected hooks fail-closed with a clear diagnostic.

## Hook contract

Each invocation receives a bounded structured payload such as:

```text
hook_event
session_id
repository_identity
cwd
canonical_tool_id?
effect_classes?
affected_paths?      # bounded and sanitized
success?              # for post/error hooks
```

Do not include source contents, prompts, secrets, raw tool output, bearer tokens, or unbounded command text by default.

Each hook returns a small structured decision/result:

```text
continue | block | request_approval
reason_code?
public_message?
```

Only lifecycle points where decisions make sense may block. Post hooks cannot retroactively claim a failed/mutated action never occurred.

## Ordering

- hook execution is deterministic and sequential;
- define stable ordering when multiple handlers match;
- hard policy checks occur before any hook can widen behavior;
- a blocking hook wins over allow decisions;
- failures have explicit per-event semantics; security hooks fail closed, cosmetic hooks may fail open only when explicitly classified as non-blocking;
- prevent recursive hook invocation unless a future explicit design adds it.

## Use cases to prove

1. format a changed source file after `file_edit`/`apply_patch`;
2. run targeted lint/diagnostics after file mutation;
3. block writes to a repository-defined protected generated file;
4. inject bounded repository state at `session_start` without editing the model prompt manually;
5. prevent agent stop when deterministic required validation is still failing;
6. notify/refresh LSP sessions after file changes;
7. emit sanitized lifecycle telemetry.

## Phases

### PHASE-01 — event/security contract

- [x] Define event enum, payloads, decisions, ordering, recursion policy, timeout/output limits.
- [x] Integrate Plan-039D deny/ask/allow precedence so hooks cannot widen authority.
- [x] Define trusted-repository enablement semantics.

### PHASE-02 — command-hook runner

- [x] Direct argv only; no implicit shell.
- [x] Reuse Bubblewrap/process execution primitives with stricter defaults where possible.
- [x] Bound duration/output/process tree.
- [x] Clear environment; expose only approved minimal variables.
- [x] Cancel cleanly with parent session.

### PHASE-03 — `pre_tool_use` / `post_tool_use` / `tool_error`

- [x] Match by canonical tool/effect class using structured fields.
- [x] Allow pre-hook block/request-approval only within hard-policy ceiling.
- [x] Keep post/error payloads sanitized.
- [x] Prove hook failure semantics.

### PHASE-04 — file-change/LSP integration

- [x] Fire `after_file_change` only after successful committed mutation.
- [x] Include bounded changed-path metadata.
- [x] Refresh LSP state from Plan 039C.
- [x] Prove formatter/check hook can run without recursive tool loops.

### PHASE-05 — session/stop lifecycle

- [x] Add `session_start` and `pre_agent_stop`.
- [x] Keep injected context bounded.
- [x] Allow deterministic repository completion gates without infinite stop loops.

### PHASE-06 — subagent lifecycle

- [ ] Add `subagent_stop` after 039F exists (dependency-gated; not started).
- [ ] Ensure child identity/policy is represented without leaking child context.
- [ ] Parent remains responsible for accepting/rejecting child result.

### PHASE-07 — hostile configuration acceptance

Test malformed configs, external executable targets, shell indirection, huge outputs, timeouts, protected paths, network requests, recursive invocations, repo identity changes, and hooks attempting to override a deny.

## Non-goals

- generic workflow engine;
- CI replacement;
- arbitrary remote webhooks;
- secret injection framework;
- model-authored hooks that silently become executable;
- automatic hook trust for unknown repositories.

## Acceptance criteria

- [x] Hook events are deterministic, bounded, and documented.
- [x] Hook config is vendor-neutral and trust-scoped by repository identity.
- [x] Hooks cannot widen hard policy or access protected credentials by default.
- [x] File-change hooks integrate with LSP/format/validation safely.
- [x] Stop hooks cannot create unbounded loops.
- [x] Sanitized hook telemetry exists without raw payload leakage.
- [x] Mandatory repository and adversarial acceptance checks pass locally; independent verification remains pending.
