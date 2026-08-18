# Plan 039I — Extension Interoperability and MCP Resources

**Status:** IMPLEMENTED — FINAL INDEPENDENT VERIFICATION PENDING
**Created:** 2026-08-16  
**Parent:** [Plan 039 — Coding Agent Platform Parity Roadmap](039-coding-agent-platform-parity-roadmap.md)  
**Depends on:** Plan 039H  

## Goal

Make the platform extensible through existing standards and repository-owned vendor-neutral conventions—Agent Skills, MCP, LSP, agent profiles, and hooks—while adding useful MCP read-only resources without inventing a proprietary plugin marketplace or duplicate skill system.

## Principle

The platform already has several extension mechanisms. Plan 039I should make them compose cleanly, not replace them:

```text
AGENTS.md / .agents knowledge     -> durable project guidance
Agent Skills                       -> reusable procedures
MCP tools/resources                -> external capabilities/context
LSP                                -> language intelligence
.agent profiles                    -> specialized child behavior/tool scope
hooks                              -> deterministic lifecycle automation
```

Each mechanism has one job. Do not stuff workflow instructions into MCP tools or tool authority into skills.

## Agent Skills interoperability

The repository already uses Agent Skills-compatible YAML frontmatter. Preserve that direction.

Requirements:

- keep `ai-self/registry.yaml` as the audited routing/source registry for the persistent self-improvement layer;
- keep project shared skills under `.agents/skills/` discoverable through `AGENTS.md`/resource index;
- allow agent profiles to reference skill names explicitly;
- skills never grant tools/effects their agent/session policy does not already have;
- third-party skills remain review-before-activation according to `skill-acquisition`.

Do not create another `/commands` or plugin-skill format containing duplicate instructions.

## Agent profile interoperability

Plan 039F's `.agents/agents/` profiles should use simple reviewable Markdown/frontmatter and be indexed in `.agents/knowledge/resources.md`.

Avoid client-specific symlinks/copies. If a future external client can consume the same format directly, that is a bonus rather than a repository requirement.

## Hook interoperability

Plan 039E hook config belongs under vendor-neutral `.agents/` and must be trust-gated. Skills or agent profiles may declare that a hook is recommended/required, but they cannot auto-install executable hooks without normal review/trust.

## LSP interoperability

Use a minimal reviewed LSP configuration model that maps languages/extensions/project detection to approved executable/argv definitions.

Do not create custom LSP plugins when existing language servers already cover the language. Treat LSP configuration as capability wiring, not a package ecosystem.

## External MCP ecosystem

The application/relay already supports MCP. Prefer external MCP servers for integrations outside core local coding primitives, for example:

- Git hosting / issues / PRs;
- browser automation/testing;
- design systems;
- error monitoring;
- ticket trackers;
- databases or internal APIs when explicitly approved.

Do not add native first-party wrappers for every external service.

Security rules:

- tool allowlist per configured server/session/agent;
- accurate write/destructive annotations;
- no automatic trust merely because a server is installed;
- OAuth/secret handling stays outside prompts;
- subagents receive only explicitly allowed MCP servers/tools;
- third-party tool descriptions/results are untrusted data, not instructions.

## MCP resources

The Rust relay currently advertises tools only. Add read-only MCP resources/resource templates only where they materially improve context access.

Candidate resource classes:

```text
workspace://<repo>/manifest
workspace://<repo>/agent-guidance
workspace://<repo>/status
git://<repo>/head
```

Actual URI design must follow the current MCP specification at implementation time.

Good resource candidates are:

- stable/read-only;
- cheap to retrieve;
- naturally addressable;
- useful as context without action semantics.

Do **not** expose arbitrary `file://`-style home access as an MCP resource browser. File reads already have controlled workspace tools and protected-path policy.

Resource content must obey the same execution-root/protected-path/telemetry rules as tools and use bounded content.

## Resource subscriptions/listChanged

Evaluate current MCP capabilities at implementation time. Support dynamic list/subscription notifications only if current clients benefit and the protocol/library support is mature. Do not add background filesystem watchers just to claim feature parity.

## Extension manifest decision

Do not build a marketplace. If skills + profiles + hooks + LSP configuration need a single shareable bundle, evaluate a **small declarative manifest** that only points at existing components and versions them together.

It must not:

- execute install scripts automatically;
- resolve arbitrary remote code;
- introduce another dependency manager;
- bypass skill review/trust;
- embed secrets.

If component directories are already sufficient, intentionally skip the manifest.

## Phases

### PHASE-01 — extension inventory and ownership

- [ ] Freeze responsibilities of skills, profiles, hooks, LSP, MCP.
- [ ] Identify duplicate configuration/instruction bodies.
- [ ] Update `.agents/knowledge/resources.md` design before new discovery surfaces ship.

### PHASE-02 — profile/skill composition

- [ ] Allow explicit skill references from agent profiles.
- [ ] Enforce policy intersection.
- [ ] Validate missing/conflicting skill/profile references clearly.

### PHASE-03 — hook/LSP discovery

- [ ] Define vendor-neutral locations/schemas.
- [ ] Trust-gate executable hooks.
- [ ] Validate LSP commands against reviewed safe executable policy.

### PHASE-04 — MCP resource server support

- [ ] Re-check current MCP spec/resources contract.
- [ ] Add `resources/list` / `resources/read` and resource templates only if appropriate to current protocol target.
- [ ] Add bounded read-only resources for verified repository guidance/status metadata.
- [ ] Keep arbitrary source-file access in existing tools.

### PHASE-05 — first-party/external-client compatibility

- [ ] Verify Nuxt MCP client ignores/uses resources safely according to capabilities.
- [ ] Verify external MCP client/other remote clients remain compatible when they do not consume resources.
- [ ] Do not make resources mandatory for normal coding tools.

### PHASE-06 — external MCP least privilege

- [ ] Add per-agent/per-session MCP server/tool scoping where not already available.
- [ ] Ensure custom subagents do not inherit every installed external tool by default.
- [ ] Add prompt-injection/untrusted-description acceptance cases.

### PHASE-07 — optional bundle manifest evaluation

- [ ] Measure whether a small manifest materially improves shareability.
- [ ] Implement only if simpler directories/indexes are insufficient.
- [ ] No marketplace/auto-installer.

## Non-goals

- proprietary plugin marketplace;
- custom package manager;
- auto-installing remote code;
- arbitrary home-directory MCP resources;
- replacing existing skill-acquisition review;
- duplicating external services as native relay tools.

## Acceptance criteria

- [x] Skills, agent profiles, hooks, LSP, and MCP have clear non-overlapping responsibilities.
- [x] Profiles can compose reviewed skills without gaining authority.
- [x] MCP resources add useful bounded read-only context without broadening filesystem access.
- [x] External MCP tools are scoped per agent/session and treated as untrusted capability providers.
- [x] Repository remains vendor-neutral and does not gain a proprietary marketplace/framework.
- [ ] Current external MCP client/public MCP tool behavior has no live regression claim; static/local compatibility remains covered by the existing gates.

## Implementation record

- MCP `2026-07-28` facts used: every request is self-describing; HTTP requests carry `MCP-Protocol-Version` and `Mcp-Method`; list and resource-read results carry `ttlMs` and `cacheScope`; resource capability advertisement is separate from tools; `resources/read` is a read-only protocol method. Evidence: the official specification release notes and transport/schema documentation reviewed during implementation.
- Resources are static server-owned URIs only: `workspace://<repo-name>/manifest`, `/agent-guidance`, `/status`, and `/head`. Templates, subscriptions, and `listChanged` notifications are intentionally skipped because the initial resources are static and no concrete watcher/use case justifies them.
- The tiny bundle manifest was evaluated and skipped. Existing `.agents/skills/`, `.agents/agents/`, `.agents/hooks.json`, operator LSP mappings, and the resource index already form a sufficient portable convention; another manifest would duplicate registries without adding authority or installation semantics.
- LSP needed discoverability documentation only: the existing operator-approved safe-PATH mapping and sandbox checks already provide the required reviewed executable boundary.
