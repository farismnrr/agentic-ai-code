# Plan 039D — Capability Policy, Approvals, and Protected Boundaries

**Status:** PLANNED  
**Created:** 2026-08-16  
**Parent:** [Plan 039 — Coding Agent Platform Parity Roadmap](039-coding-agent-platform-parity-roadmap.md)  
**Depends on:** Plan 039C  

## Goal

Turn the existing tool-level approval behavior and relay sandbox controls into one coherent coding-agent capability policy: hard protected boundaries at the relay, argument-aware deny/ask/allow decisions in the first-party client, explicit command/network risk handling, and consistent policy metadata across native/MCP tools.

## Current state

- `ai-self/policies/default.yaml` already expresses durable external MCP client/MCP operating approval policy for repository work.
- The Nuxt application persists conversation-scoped MCP approval decisions as `always` / `never` and uses AI SDK tool-approval mechanisms.
- Current application MCP decisions are primarily per-tool ID, not a general argument-aware policy language.
- The Rust relay uses OAuth `relay.coding`, tool annotations, execution-root containment, safe PATH, and Bubblewrap.
- Terminal Bubblewrap masks common credential stores under a home-scoped execution root.
- Native Plan-038 filesystem tools currently do not share that credential-store protection; containment alone means a path under the home execution root is technically addressable.
- Bubblewrap process isolation currently does not by itself provide a documented per-call network permission model.

## Core distinction

This plan must not collapse three different concerns:

### Relay hard policy

Server-enforced, non-interactive constraints for every MCP client:

- execution root;
- protected paths;
- process sandbox;
- safe executable discovery;
- Docker/Tailscale explicit opt-ins;
- maximum resource limits;
- network mode constraints;
- OAuth/owner/scope.

A direct external MCP client cannot click the Nuxt approval UI, so anything that must *always* be forbidden belongs here.

### First-party approval policy

Interactive policy evaluated by the Nuxt app before approved tool execution:

- allow automatically;
- ask once;
- ask every time;
- deny;
- remember a narrowly scoped rule where safe.

### External client approval

external MCP client/other MCP clients decide their own prompts based on their product policy and MCP metadata. The relay cannot assume those prompts exist. It still enforces hard server policy.

## Capability/effect taxonomy

Create one reusable effect model rather than ad-hoc booleans across layers. Candidate categories:

```text
workspace_read
workspace_write
workspace_delete
git_read
process_exec
network_read
network_write
external_mutation
privileged_bridge
```

A tool may have multiple effects. The taxonomy should coexist with MCP `readOnlyHint`, `destructiveHint`, `idempotentHint`, and `openWorldHint` rather than replacing standard annotations.

## Rule model

For the first-party approval policy, use explicit precedence:

```text
deny > ask > allow > default
```

Rules may match safe, top-level structured properties such as:

- canonical tool ID;
- effect class;
- contained path/glob;
- Git operation mode;
- target domain/host for dedicated network tools;
- executable and direct argv prefix for terminal calls;
- subagent role/isolation mode later in 039F/039G.

Do not implement fragile generic string matching that claims to parse arbitrary shell safely.

Opaque execution should escalate:

- `sh -lc`, `bash -c`, interpreters with inline code, `eval`, arbitrary script runners, complex wrappers, and unparseable commands are not eligible for broad auto-allow merely because the first token looks safe.

## Protected paths

Create one canonical protected-path policy shared by native workspace tools and terminal sandbox setup.

Initial protected classes should cover existing masked credential stores and any other verified high-risk locations under the home execution root, including at least current terminal protections:

- `.ssh/`
- `.aws/`
- `.config/gcloud/`
- `.docker/`
- `.kube/`
- `.npmrc`
- `.netrc`
- `.pypirc`
- `.cargo/credentials`
- `.cargo/credentials.toml`

During implementation, audit additional common credential stores actually relevant to the host/toolchains. Keep the set narrowly justified; do not enumerate private user data unnecessarily.

Requirements:

- native read/search/list/write/edit/patch/Git/LSP helpers must not bypass protected paths;
- symlink aliases to protected targets remain protected;
- terminal masks and native policy derive from one authoritative configuration/model where feasible;
- external access to secrets remains explicit/manual according to higher-level policy, not silently enabled by a tool argument;
- `.env.example` must not be accidentally blocked merely because `.env` is sensitive; rules need precise semantics.

## Permission modes

For first-party agent mode, design simple modes comparable to industry patterns without cloning names if not useful:

1. **Plan / Read-only** — reads, searches, Git read, LSP navigation/diagnostics; no workspace mutation or general execution except reviewed read-only commands.
2. **Workspace** — workspace edits allowed according to policy; risky process/network actions still ask.
3. **Autonomous sandboxed** — low-risk workspace actions auto-approved inside hard boundaries; high-risk/destructive/external actions still stop.
4. **Manual** — default prompt-oriented mode for users who want explicit control.

Never provide a UI mode that bypasses the relay's hard security boundary.

## Command risk classification

Avoid attempting to prove arbitrary shell semantics. Use conservative classes:

- fixed direct-argv read-only commands with reviewed subcommands/options;
- known local build/check/test commands that can write generated artifacts but stay inside workspace;
- package installation / network-capable operations;
- Git mutation;
- opaque shell/interpreter execution;
- privileged bridge commands such as Docker when enabled;
- destructive filesystem/process commands.

Unknown = ask/deny according to mode, never auto-safe.

## Network policy

Current coding-agent security practice treats network policy as separate from filesystem sandboxing. This plan should add an explicit relay network contract without pretending Bubblewrap alone filters domains.

Required design work:

- identify which tools inherently use network (`http_fetch`, `web_search`, package managers, Git remote commands, arbitrary terminal programs);
- distinguish dedicated HTTP/domain-aware tools from opaque terminal network access;
- introduce a clear operator/session policy for terminal network access;
- prefer network-off sandbox execution when a command does not require network, if this can be implemented without breaking local toolchains;
- require explicit authority for host/network-enabled terminal execution in stricter modes;
- do not build a transparent proxy/firewall subsystem unless a simpler OS-level mechanism cannot satisfy the requirement;
- preserve current local development compatibility through an explicit migration/configuration path rather than silently breaking builds.

Dedicated HTTP tools can support host/domain policy more precisely than Bash/curl string rules.

## Approval UX contract

Approval prompts should show concise structured information, not just a tool name:

- what capability/effect is requested;
- executable/subcommand or target path/domain;
- working directory/repository;
- whether network is requested;
- whether a protected/destructive/external boundary is involved;
- low/medium/high risk classification derived from deterministic policy facts, not model-generated claims;
- available choices and exact scope of remembered approval.

A remembered approval must show whether it is scoped to call, session, conversation, repository, tool, command prefix, or domain.

## Phases

### PHASE-01 — policy inventory and threat model

- [ ] Map current `ai-self` policy, Nuxt tool approvals, MCP annotations, relay sandbox, credential masking, Docker/Tailscale opt-ins.
- [ ] Identify duplicate/inconsistent policy sources.
- [ ] Freeze effect taxonomy and hard-vs-interactive responsibility split.

### PHASE-02 — shared protected-path policy

- [ ] Extract canonical protected-path model.
- [ ] Apply it to native workspace/Git/LSP operations.
- [ ] Keep terminal masking behavior equivalent or stronger.
- [ ] Add symlink/protected-path negative acceptance coverage.

### PHASE-03 — application rule engine

- [ ] Implement deny/ask/allow/default precedence.
- [ ] Support safe matchers for tool/effect/path/domain/direct-argv fields.
- [ ] Migrate existing `always` / `never` decisions without silent privilege expansion.
- [ ] Define session vs persisted rule scope.

### PHASE-04 — permission modes and UI

- [ ] Add simple mode selection and clear semantics.
- [ ] Make Plan/read-only mode genuinely non-mutating.
- [ ] Render structured risk/affected-scope approval prompts.
- [ ] Provide policy/rule management without exposing raw secrets.

### PHASE-05 — terminal command policy

- [ ] Create conservative fixed/direct-argv classifiers.
- [ ] Treat shells/interpreters/wrappers conservatively.
- [ ] Ensure compound/opaque commands cannot inherit a safe prefix approval accidentally.
- [ ] Keep shell syntax available through explicit shell invocation, but require appropriate authority.

### PHASE-06 — network boundary

- [ ] Add explicit network capability metadata/config.
- [ ] Implement the simplest enforceable network-off/host-network distinction supported by the existing Linux/Bubblewrap architecture.
- [ ] Add domain-aware policy to dedicated HTTP paths where practical.
- [ ] Verify package/build/Git workflows and document when network approval is required.

### PHASE-07 — external MCP truthfulness

- [ ] Ensure standard MCP annotations accurately reflect effects.
- [ ] Do not expose first-party approval state as if it protected direct remote calls.
- [ ] Verify external MCP client/other-client calls remain subject to relay hard policy.

### PHASE-08 — security falsification

Attempt bypasses with:

- protected path direct/relative/absolute access;
- symlink aliases;
- native workspace reads/searches;
- LSP location responses;
- Git path filters;
- `sh -lc` / interpreter indirection;
- command wrappers;
- network commands hidden behind shells;
- Docker/Tailscale opt-ins;
- stale/overbroad remembered rules;
- tool-name sanitization collisions;
- malformed policy config.

## Acceptance criteria

- [ ] Native and terminal operations enforce one coherent protected-path boundary.
- [ ] First-party approvals can be argument/effect aware with deny > ask > allow precedence.
- [ ] Unknown/opaque execution is never silently auto-classified safe.
- [ ] Network capability is explicit and enforceable at least at off-vs-enabled boundary for terminal execution.
- [ ] Dedicated HTTP/domain policy does not rely on fragile shell parsing.
- [ ] External MCP clients remain protected by hard relay policy independent of Nuxt UI approvals.
- [ ] No existing OAuth/Bubblewrap/Docker/Tailscale boundary is weakened.
- [ ] Mandatory commit/security/black-box verification passes.
