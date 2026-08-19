# Plan 044C — GitHub Security Alert Visibility

**Status:** SOURCE IMPLEMENTED / DETERMINISTIC VERIFICATION PASS / MERGE PENDING
**Parent:** [Plan 044](044-github-repository-operations-security-roadmap.md)
**Depends on:** Plan 044B source merged / live verification pending

## Goal

Add read-only GitHub security-alert visibility for Dependabot, code scanning, and secret scanning so agents can detect and investigate repository vulnerabilities through bounded structured MCP tools without exposing literal secrets or gaining alert-dismissal authority.

## Success criteria

The source implementation exposes exactly these seven tools:

- `dependabot_alert_list`
- `dependabot_alert_get`
- `code_scanning_alert_list`
- `code_scanning_alert_get`
- `secret_scanning_alert_list`
- `secret_scanning_alert_get`
- `secret_scanning_alert_locations`

All tools are read-only model-facing operations. Security-alert mutations remain absent.

## Scope

### In scope

- repository-scoped Dependabot alerts;
- repository-scoped code-scanning alerts;
- repository-scoped secret-scanning alerts;
- bounded state/severity/ecosystem/tool filters where current GitHub API supports a typed safe equivalent;
- one-alert detail for each alert family;
- secret-scanning location metadata sufficient to identify the affected repository location;
- typed provider-status classification for unavailable/permission/not-found cases without raw provider error bodies;
- deterministic secret/PII canary acceptance;
- live read verification where the operator token/repository has access.

### Out of scope

- organization-wide alert APIs;
- alert dismissal/resolve/reopen;
- Dependabot dismissal comments/reasons;
- code-scanning dismissal state/reason;
- secret-scanning resolution/validity/assignment;
- push-protection bypass;
- security-and-analysis enable/disable;
- SARIF upload;
- custom secret patterns;
- repository security policy/admin settings;
- exposing literal secret values;
- returning security-alert metadata that contains owner email/name/id unless explicitly allowlisted in a future plan.

## External API facts that must remain true or be revalidated

At implementation start, re-check official GitHub REST documentation and pin the reviewed API version used by the fixed adapter. Planning baseline on 2026-08-19:

- Dependabot read endpoints require Dependabot-alert read permission for fine-grained tokens; classic tokens generally need `security_events` for private repositories.
- code-scanning alert reads require Code-scanning-alert read permission; classic tokens generally need `security_events` for private repositories.
- secret-scanning reads require Secret-scanning-alert read permission and, for repository alert access, repository/organization administrator authority according to current GitHub documentation.
- both secret-scanning list and get endpoints default to exposing literal secret values unless `hide_secret=true` is supplied.
- secret-scanning response schemas contain a `secret` field and may contain metadata such as owner email/name/id; model-visible normalization must not rely on GitHub masking alone.

## Security decisions

### SD-001 — Fixed REST templates only

Security alerts have no sufficiently complete high-level `gh` command family. Use an internal fixed GitHub REST adapter through the existing credential-isolated `gh` boundary (or an equally narrow reviewed transport), with:

- method selected by code, never model input;
- host fixed to GitHub validated by existing remote identity;
- path generated only from validated owner/repo + positive alert ID;
- fixed `Accept` and reviewed API-version header;
- reviewed query-key allowlist;
- no arbitrary `gh api` flags, path, form/body, jq/template, pagination URL or header passthrough.

### SD-002 — Normalize before exposure

Never return provider response JSON directly. Deserialize into narrow structs and construct separate model DTOs containing only reviewed fields.

### SD-003 — Secret-scanning double barrier

Every secret-scanning list/get request must force `hide_secret=true`. After parsing, the `secret` field is discarded unconditionally and must not exist in any public result type. Deterministic fixtures must deliberately return a literal canary despite the request flag and prove the normalizer still strips it.

### SD-004 — Metadata minimization

Secret-scanning metadata arrays are dropped by default. Preserve only safe operational booleans/enums and repository-location facts needed for remediation. Owner email/name/id and provider-secret metadata do not belong in the model-facing result.

### SD-005 — Read-only means no alert disposition

Do not implement update endpoints in this plan. Remediation occurs through normal source/dependency branch + PR workflow. A future plan may consider alert dismissal only with separately reviewed reasons, approvals and evidence requirements.

## PHASE-01 — Fixed security API transport contract

**Goal:** Extend the privileged forge bridge to support enumerated GitHub REST reads without creating a generic API escape hatch.

**Dependencies:** none

### TASK-001 — Implement fixed GitHub REST request variants

**Outcome:** Security modules can perform reviewed GET requests and receive bounded status/body bytes through one credential-isolated path.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge_process.rs`
- Modify: `packages/rust-tools/application/src/git/forge/common.rs`
- Create/modify as appropriate: `packages/rust-tools/application/src/git/forge/security.rs`

**Steps:**
- [ ] Define internal request variants/enums for the exact Plan-044 security endpoints rather than accepting arbitrary method/path strings from tool arguments.
- [ ] Construct endpoint path from validated GitHub owner/repository and numeric alert identifier only.
- [ ] Pin/review the GitHub API version and JSON accept header at the transport boundary.
- [ ] Support bounded query parameters only through typed fields owned by each request variant.
- [ ] Capture provider HTTP status separately from body without exposing raw headers/body on error.
- [ ] Keep the existing cleared environment, safe PATH, credential forwarding, timeout, process-group cleanup and output caps.
- [ ] Map provider status into bounded classifications (`forbidden`, `not_found_or_unavailable`, `rate_limited`, `service_unavailable`, `provider_failure`) without echoing provider error text.

**Validation:**
- fake `gh` fixture asserts exact endpoint/method/header/query construction and rejects any attempt to influence host/path/method through tool input.
- malformed provider status/body and over-limit output fail closed.

**Commit boundary:** `feat(044c): add fixed github security api reads`

## PHASE-02 — Dependabot visibility

**Goal:** Provide actionable dependency vulnerability summaries without raw advisory/provider payloads.

**Dependencies:** PHASE-01

### TASK-002 — Implement Dependabot list/get

**Outcome:** Agents can identify affected package/dependency and advisory severity/range for one repository.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/security.rs`
- Modify: `packages/rust-tools/application/src/git.rs`

**Steps:**
- [ ] Implement `dependabot_alert_list` with bounded result count and typed state/severity/ecosystem filters only if current API supports them safely.
- [ ] Implement `dependabot_alert_get(alert_number)` for a positive alert number.
- [ ] Normalize only remediation-relevant fields: alert number/state, dependency package/ecosystem/manifest/scope, advisory GHSA/CVE identifiers where present, severity, vulnerable range, patched version when present, created/updated/dismissed/fixed timestamps, and validated GitHub alert URL.
- [ ] Omit raw advisory descriptions, references arrays, CVSS vectors or other large fields unless a concrete remediation need justifies a bounded field.
- [ ] Bound all strings/arrays and reject provider repository/URL identity mismatch.

**Validation:**
- fixtures cover open/fixed/dismissed states, missing patched version, malformed severity, long fields, disabled/unavailable feature and permission errors.

**Commit boundary:** `feat(044c): add dependabot alert visibility`

## PHASE-03 — Code-scanning visibility

**Goal:** Surface static-analysis findings with enough location/rule context for source investigation.

**Dependencies:** PHASE-02

### TASK-003 — Implement code-scanning list/get

**Outcome:** Agents can identify rule, severity, tool, branch/ref and most recent affected location without raw SARIF access.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/security.rs`

**Steps:**
- [ ] Implement `code_scanning_alert_list` with bounded state/severity/tool/ref filters only when typed and useful.
- [ ] Implement `code_scanning_alert_get(alert_number)`.
- [ ] Normalize alert number/state/dismissal classification, rule ID/name/security severity/description summary with strict length cap, tool name/version, most-recent-instance ref/state/commit SHA and repository-relative location path + line/column range when present.
- [ ] Validate commit SHA and reject absolute/traversal-style location paths; normalize provider paths as repository-relative evidence, not filesystem authority.
- [ ] Do not expose raw SARIF, fingerprints, provider analysis payloads, arbitrary rule tags or full message traces by default.

**Validation:**
- fixtures cover warning/error/security severity, missing location, PR refs, malformed paths/SHA, oversized rule text and provider URL mismatch.

**Commit boundary:** `feat(044c): add code scanning alert visibility`

## PHASE-04 — Secret-scanning visibility with literal-secret suppression

**Goal:** Make secret leaks investigable without ever returning the detected credential itself.

**Dependencies:** PHASE-03

### TASK-004 — Implement secret-scanning list/get

**Outcome:** Agents can see alert type/state/validity/bypass/public-leak facts while literal secrets and PII metadata remain absent.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/security.rs`

**Steps:**
- [ ] Implement `secret_scanning_alert_list` with fixed `hide_secret=true`, bounded result count, and only typed safe filters such as state/validity/secret type where needed.
- [ ] Implement `secret_scanning_alert_get(alert_number)` with fixed `hide_secret=true`.
- [ ] Provider-deserialization type may accept a `secret` field only so it can be discarded; public DTO must not define it.
- [ ] Drop provider metadata arrays, owner-email/name/id, raw resolution comments and bypass request comments by default.
- [ ] Normalize safe fields: number/state/resolution classification, secret type/display name, validity, publicly-leaked boolean, multi-repo boolean, push-protection-bypassed boolean/timestamp, created/resolved timestamps and validated alert URL.
- [ ] Ensure any secret-shaped strings appearing unexpectedly in other provider fields pass the shared credential redactor or cause strict field omission.
- [ ] Deterministic provider fixture must return a known literal canary in `secret` despite `hide_secret=true`; assert the canary is absent from serialized tool result, errors and telemetry/stderr evidence.

**Validation:**
- canary fixture contains fake token/secret/email metadata and proves none leak through result/errors/logs.
- schema/result inspection proves public DTO has no `secret` property.

**Commit boundary:** `feat(044c): add secret scanning alert visibility`

### TASK-005 — Implement secret-scanning location list

**Outcome:** Agents can locate the affected repository object without accessing the secret value.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/security.rs`

**Steps:**
- [ ] Implement `secret_scanning_alert_locations(alert_number)` through the fixed locations endpoint.
- [ ] Cap location count and normalize only reviewed location kinds.
- [ ] For commit/blob locations, return validated commit/blob SHA, repository-relative path and bounded line/column coordinates.
- [ ] Reject absolute paths, parent traversal, malformed SHAs, provider-owned arbitrary URLs and unknown oversized nested payloads.
- [ ] Do not return snippets, secret values, diff content or direct signed download URLs.

**Validation:**
- fixture covers multiple locations, truncation, malformed path/SHA and secret canary nested in unreviewed provider fields.

**Commit boundary:** `feat(044c): add secret scanning locations`

**Phase exit criteria:**
- [ ] literal secret canary cannot cross any public result boundary.
- [ ] alert location is sufficient to hand off to normal source investigation tools.

## PHASE-05 — Catalog, policy and security acceptance

**Goal:** Register all seven reads as sensitive privileged network capabilities and freeze their safe schema.

**Dependencies:** PHASE-04

### TASK-006 — Expose security-alert tools through MCP safely

**Outcome:** MCP schemas, policy effects and presentation summaries remain consistent and do not make security reads look like ordinary workspace reads.

**Files:**
- Modify: `packages/rust-tools/interfaces/src/mcp/catalog/forge.rs`
- Modify: `packages/rust-tools/application/src/hooks/policy.rs`
- Modify: `shared/utils/capability-policy.ts`
- Modify if needed: `app/utils/tool-presentation.ts`
- Create: `scripts/verify-044c-security-alerts.sh`

**Steps:**
- [ ] Add strict schemas with positive alert IDs, reviewed filter enums and hard result caps.
- [ ] Mark all seven tools read-only + open-world + non-destructive.
- [ ] Map all seven to `network_read + privileged_bridge` in both policy owners so they remain approval-sensitive privileged reads rather than auto-approved workspace reads.
- [ ] Ensure safe summaries show alert family/number/state intent only, not secret types combined with any hidden value or provider metadata.
- [ ] Assert schemas contain no arbitrary API host/path/method/header/body controls.
- [ ] Add canaries proving literal secrets, auth tokens, owner email metadata and raw provider errors never appear in public result/error surfaces.

**Validation:**
- `bash scripts/verify-044c-security-alerts.sh` → PASS.
- prior 040/044A/044B acceptance → PASS.
- `cargo test --workspace` → PASS.
- `pnpm verify:commit` → PASS.

**Commit boundary:** `feat(044c): expose github security alert reads`

## PHASE-06 — Documentation and merge handoff

### TASK-007 — Integrate read-only security visibility

**Outcome:** Source is merged with explicit permission/availability limits and no false claim that every repository/token can access every security feature.

**Files:**
- Modify: `packages/rust-tools/README.md`
- Modify: `docs/chatgpt.md`
- Modify: this plan and parent Plan 044
- Modify `.agents/memories/README.md` with the durable secret-scanning double-barrier invariant.

**Steps:**
- [ ] Document required GitHub feature/token permissions at a capability level without storing credentials/scopes from the operator environment.
- [ ] Document `hide_secret=true` + unconditional field omission invariant.
- [ ] State that disabled/unlicensed/unpermitted security features may return bounded unavailable results and are not implementation failures by themselves.
- [ ] Run mandatory closeout review and gates.
- [ ] Deliver via short-lived implementation branch + PR to `main`.
- [ ] Mark 044C `MERGED / LIVE VERIFICATION PENDING` until 044D final deployment/live proof.

**Validation:**
- exact merged source passes deterministic secret canary and repository gate.

**Commit boundary:** normal implementation PR squash merge to `main`.

## Risks and rollback

- **Literal secret disclosure:** double barrier plus deliberate hostile provider fixture. Any canary leak is a P0 blocker; do not ship a partial workaround.
- **PII metadata disclosure:** allowlist normalized fields and drop metadata arrays by default.
- **Token lacks security permission:** return bounded unavailable/forbidden classification; do not inspect token contents, credential files or broaden scopes automatically.
- **Feature unavailable on repo/plan:** treat as runtime capability absence, not reason to weaken parsing/security.
- **Provider error leak:** status classification only; raw GitHub error JSON/stderr stays private/ignored.
- **Path confusion:** repository locations are evidence strings, never automatically authorized filesystem paths.

## Final 044C acceptance criteria

- [ ] Seven security-read tools exist exactly once.
- [ ] Dependabot/code/secret alert outputs are typed and bounded.
- [ ] every secret-scanning list/get request uses `hide_secret=true`.
- [ ] public result types have no literal-secret field.
- [ ] hostile provider fixture cannot leak secret/PII canaries through result/errors/logs.
- [ ] no security alert mutation exists.
- [ ] no arbitrary API passthrough exists.
- [ ] ordinary terminal credential isolation remains intact.
- [ ] deterministic 044C acceptance passes.
- [ ] `cargo test --workspace` passes.
- [ ] `pnpm verify:commit` passes.
- [ ] source PR merges to `main`.
- [ ] live security-read proof remains pending until 044D deployment.

## Handoff

After 044C is merged source-clean, continue to [Plan 044D](044d-controlled-actions-mutations-and-closure.md).
