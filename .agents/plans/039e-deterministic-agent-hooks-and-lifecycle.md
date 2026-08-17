# Plan 039E — Deterministic Agent Hooks and Lifecycle

**Status:** PLANNED  
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

- [ ] Define event enum, payloads, decisions, ordering, recursion policy, timeout/output limits.
- [ ] Integrate Plan-039D deny/ask/allow precedence so hooks cannot widen authority.
- [ ] Define trusted-repository enablement semantics.

### PHASE-02 — command-hook runner

- [ ] Direct argv only; no implicit shell.
- [ ] Reuse Bubblewrap/process execution primitives with stricter defaults where possible.
- [ ] Bound duration/output/process tree.
- [ ] Clear environment; expose only approved minimal variables.
- [ ] Cancel cleanly with parent session.

### PHASE-03 — `pre_tool_use` / `post_tool_use` / `tool_error`

- [ ] Match by canonical tool/effect class using structured fields.
- [ ] Allow pre-hook block/request-approval only within hard-policy ceiling.
- [ ] Keep post/error payloads sanitized.
- [ ] Prove hook failure semantics.

### PHASE-04 — file-change/LSP integration

- [ ] Fire `after_file_change` only after successful committed mutation.
- [ ] Include bounded changed-path metadata.
- [ ] Refresh LSP state from Plan 039C.
- [ ] Prove formatter/check hook can run without recursive tool loops.

### PHASE-05 — session/stop lifecycle

- [ ] Add `session_start` and `pre_agent_stop`.
- [ ] Keep injected context bounded.
- [ ] Allow deterministic repository completion gates without infinite stop loops.

### PHASE-06 — subagent lifecycle

- [ ] Add `subagent_stop` after 039F exists.
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

- [ ] Hook events are deterministic, bounded, and documented.
- [ ] Hook config is vendor-neutral and trust-scoped by repository identity.
- [ ] Hooks cannot widen hard policy or access protected credentials by default.
- [ ] File-change hooks integrate with LSP/format/validation safely.
- [ ] Stop hooks cannot create unbounded loops.
- [ ] Sanitized hook telemetry exists without raw payload leakage.
- [ ] Mandatory repository and adversarial acceptance checks pass.
