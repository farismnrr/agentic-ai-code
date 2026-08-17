# Plan 039J — Agent UX, Observability, and Final Closure

**Status:** PLANNED  
**Created:** 2026-08-16  
**Parent:** [Plan 039 — Coding Agent Platform Parity Roadmap](039-coding-agent-platform-parity-roadmap.md)  
**Depends on:** Plan 039I  

## Goal

Integrate the Plan-039 capability family into one coherent coding-agent experience, add truthful action-level observability and review UX, perform adversarial/security/regression acceptance, update durable documentation, and close the roadmap only with real evidence.

## UX target

The first-party Nuxt agent experience should make autonomous work understandable without dumping internal reasoning.

Users should be able to see:

- current task/todo state;
- active tool/subagent/background job;
- concise tool input summary appropriate to sensitivity;
- approval/risk/effect scope when a decision is needed;
- file changes/diff preview;
- diagnostics/validation outcome;
- subagent summary and evidence;
- hook block/failure when relevant;
- context pressure/compaction state;
- cancellation controls;
- final summary of what changed and what remains unproven.

Do not expose chain-of-thought or raw provider/system internals as a UX requirement.

## Tool-call rendering

Create consistent rendering categories rather than one bespoke component per tool:

- read/search/navigation;
- Git/change inspection;
- file/patch mutation;
- execution/background job;
- network/external action;
- subagent/background agent;
- hook/policy event;
- diagnostics/validation.

Show bounded structured metadata and collapse noisy results by default.

## Diff/review UX

For file writes/edits/patches/background agent changes:

- show affected files and bounded diff;
- distinguish staged/unstaged/background-worktree state;
- indicate truncation with continuation affordance;
- show whether change was applied, preview-only, or failed preflight;
- never claim multi-file atomicity beyond implementation guarantee.

## Approval UX

Integrate Plan 039D structured policy:

- capability/effect;
- target repository/cwd;
- affected path/domain/executable;
- network requested or not;
- risk class from deterministic facts;
- one-time/session/persisted scope;
- deny/ask/allow source where useful.

Do not show secret values or unbounded command payloads.

## Subagent/background UX

- compact child card with role, task, state, elapsed/relative progress without fake percentages, tool/effect scope, cancel action;
- result summary/evidence when complete;
- background worktree/branch identity when a writer is isolated;
- parent integration remains explicit.

## Agent-native observability

Reuse existing OpenTelemetry/logging rather than introducing another event database.

Add bounded semantic telemetry for:

- agent session/turn ID;
- canonical tool ID and effect classes;
- policy outcome source (hard deny / ask / allow / client decision), without secret rule values;
- tool duration/result classification/truncation;
- hook event/outcome/duration;
- LSP server type/capability/outcome (not source contents);
- subagent role/state/budget consumption summary;
- background task/worktree lifecycle;
- context compaction/budget class;
- cancellation/timeouts.

Never emit:

- prompt contents;
- source/file contents;
- patch contents;
- raw shell arguments when they may contain secrets;
- auth tokens/cookies/headers;
- raw LSP/provider/tool errors;
- private filesystem paths when a normalized relative identifier is enough.

Use existing sanitizer/classification contracts from Plan 035.

## Security/falsification matrix

Before closure, run fresh source-level and black-box attacks across the composed system, including:

### Filesystem/protected data

- direct/relative/absolute protected paths;
- symlink aliases;
- Git/LSP/resource access to protected targets;
- patch/write through protected/symlinked parents.

### Execution/network

- opaque shell/interpreter commands;
- compound commands/wrappers;
- network hidden behind shell;
- Docker/Tailscale opt-ins;
- network-disabled execution attempting egress.

### Approval/policy

- deny vs narrower allow collision;
- stale persisted rules;
- malformed rules;
- tool-name collision/sanitization;
- subagent trying to widen parent authority;
- hook attempting to override hard deny.

### Hooks

- malicious repo hook config;
- timeout/output bomb;
- recursion;
- changed repository identity;
- hook command outside safe PATH.

### LSP

- malicious server response/location;
- crash/hang;
- stale version;
- sibling repo cross-talk.

### Subagents/background

- recursion explosion;
- cancellation leaks;
- concurrent writers to same checkout;
- worktree cleanup of dirty/unmerged state;
- child attempts Git push/merge/destructive action outside policy.

### MCP/external

- third-party tool description/result prompt injection;
- write tool incorrectly annotated read-only;
- resources exposing arbitrary home paths;
- direct external MCP client call bypassing Nuxt approval assumptions.

## End-to-end acceptance scenarios

At minimum prove these workflows in the first-party app and, where relevant, direct remote MCP:

1. **Explore unfamiliar repo** — directory/file/text/Git/LSP read-only path with no approval spam.
2. **Small fix** — inspect → edit/patch → diagnostics → targeted validation → diff review.
3. **Plan-only task** — plan subagent cannot mutate.
4. **Independent review** — review subagent returns bounded findings/evidence without changing source.
5. **Background isolated implementation** — child writes only in task worktree; parent reviews diff before integration.
6. **Protected credential attack** — read/search/LSP/Git/terminal attempts fail at hard boundary.
7. **Network-gated command** — local command runs without network; network-requiring command follows configured approval/policy path.
8. **Hook enforcement** — file change triggers deterministic formatter/check; hard-deny cannot be overridden.
9. **Large repository result** — continuation works without flooding context.
10. **Long command + cancellation** — existing Plan-037 lifecycle still works alongside new policy/hooks/subagents.
11. **Direct external MCP client MCP** — new relay tools/resources remain OAuth-protected and hard policy applies even without Nuxt approval UI.

## Documentation

Update only authoritative current docs/knowledge:

- `README.md` tool/capability summary where appropriate;
- `docs/architecture.md`;
- `docs/getting-started.md` / configuration/security docs for new operator settings;
- `docs/external-mcp.md` for remote-client truthfulness;
- `packages/relay-agent/SKILL.md`;
- `.agents/knowledge/project.md`;
- `.agents/knowledge/tooling.md`;
- `.agents/knowledge/resources.md`;
- `.agents/memories/README.md` with durable implementation decisions only;
- `ai-self` skill/lesson updates only when reusable procedural learning actually emerged.

Do not copy plan prose wholesale into durable knowledge.

## Final verification

Use repository-authoritative checks. Final closure should include at least:

```bash
pnpm verify:commit
pnpm build
pnpm build:tools
```

plus all relevant deterministic Plan-039 security/contract scripts, dependency audits when dependencies changed, and live relay/Nuxt/external MCP client acceptance appropriate to the changed surfaces.

If a live external proof cannot be executed, mark that criterion **UNPROVEN**, not passed.

## Closure review

Before marking Plan 039 closed:

- request a fresh independent subagent review focused on security, architecture, correctness, and over-engineering;
- fix all confirmed P0/P1 findings;
- reconcile docs/plan/memory/source truth;
- ensure every child plan is actually closed;
- verify worktree clean/expected and repository identity;
- follow `github-delivery` for final commit/push behavior;
- do not merge/release/deploy without the normal required approval.

## Final acceptance criteria

- [ ] 039B–039I are closed with evidence.
- [ ] First-party agent UX exposes tasks/tools/subagents/approvals/diffs/diagnostics coherently.
- [ ] Agent-native telemetry is useful but confidentiality-preserving.
- [ ] Full composed security falsification has no unresolved P0/P1 issue.
- [ ] Existing Plan-035/036/037/038 security and execution contracts have no regression.
- [ ] Direct remote MCP does not rely on first-party UI approvals for hard safety.
- [ ] Documentation and durable agent knowledge match current implementation.
- [ ] No custom vector DB/RAG/plugin marketplace/parallel agent framework was introduced without proven need.
- [ ] Mandatory local/build/security/live acceptance is passed or truthfully marked unproven.
- [ ] Master Plan 039 checklist is updated only after all evidence exists.
