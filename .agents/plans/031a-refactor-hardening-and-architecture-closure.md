# Plan 031A — Refactor Hardening and Architecture Closure

**Status: CLOSED — HARDENING PASS COMPLETE; THIRD-REVIEW CLOSURE MOVED TO PLAN 031B**  
**Created: 2026-08-13**  
**Closed administratively: 2026-08-13**  
**Parent plan: Plan 031 — Repository-wide Layered Refactor**  
**Implementation branch: `refactor/031-repository-wide-layered-refactor`**  
**Original 031A audit baseline: `b241175e131e544ba7cf922f8d5865557e3f66e3`**  
**Second deep-review baseline: `dcd2fb4`**  
**Third deep-review baseline / handoff source: `b43f1fe9cc08c2ba6df69f6407f1f37e71bb0e85`**  
**Successor plan: [`031b-final-architecture-security-and-release-closure.md`](031b-final-architecture-security-and-release-closure.md)**

---

## Closure handoff — 2026-08-13

Plan 031A is closed **administratively**, not because every historical acceptance criterion was proven complete.

The user explicitly decided to stop expanding Plan 031A after a third strict source-level review found another meaningful set of architecture/security/verification gaps. Those gaps are large and systemic enough to justify a dedicated final closure plan rather than another sequence of appended 031A phases.

This supersedes the earlier 031A instruction that said not to create Plan 031B merely to move unfinished work. Plan 031B is being created by explicit user decision because the remaining work is now a distinct final architecture/security/release-closure pass, not as a cosmetic attempt to hide unfinished acceptance criteria.

**Do not reopen Plan 031A.** All active work in this plan family now belongs to Plan 031B.

Closing 031A does **not** mean:

- the branch is merge-ready;
- all Plan 031/031A architectural acceptance is proven;
- all P0/P1 findings are resolved;
- `pnpm verify:commit` or `pnpm build` completed successfully in the previous sandbox;
- the live browser/two-user/provider runtime matrix was completed;
- the Plan 031 family achieved the requested final 10/10 standard.

Those claims may be made only if Plan 031B reaches its own Definition of Done with real evidence.

---

# What Plan 031A materially accomplished

The following improvements were implemented during Plan 031A and must be preserved by Plan 031B:

## Tenant isolation

- conversation create validates model/provider ownership;
- conversation model updates validate ownership;
- chat context reasserts conversation → model → provider same-user ownership;
- workspace association and prompt resolution became user-scoped;
- default model writes validate model ownership;
- `lastActiveWorkspaceId` was later updated to validate owned workspace before persistence;
- provider model discovery is scoped through the user's provider.

## Provider secret handling

- provider DTOs no longer return decrypted custom-header values;
- new custom header values are encrypted at rest;
- provider edit semantics preserve unchanged secret values without round-tripping plaintext;
- legacy plaintext custom headers gained a lazy upgrade path;
- API keys remain encrypted using the existing provider secret mechanism.

## Provider SSRF work

- provider SDK/discovery paths were moved onto `createSsrfSafeFetch()`;
- redirect following became manual/bounded rather than delegated blindly to native fetch;
- redirect targets are rechecked through the address policy;
- private, loopback, link-local, metadata, and mapped address classification was expanded.

The third review later found that credential containment across public cross-origin redirects and the deterministic redirect acceptance proof still need closure. Those are Plan 031B findings.

## Server refactor

- `server/api/chat.post.ts` became a materially thinner transport/composition adapter;
- `executeChatTurn()` became the main chat-turn orchestration entrypoint;
- submit/regenerate/resume semantics were moved out of database infrastructure;
- concrete provider/AI/LangGraph/MCP construction was moved further toward infrastructure;
- an explicit chat dependency object was introduced.

The third review later found that application-facing contracts are still owned/typed from infrastructure and several application files still import concrete infrastructure. Plan 031B owns the final dependency-inversion pass.

## Architecture guardrails

- `scripts/check-architecture.sh` was introduced and later expanded;
- `pnpm verify:commit` invokes the architecture checker;
- the checker protects several real boundaries including Rust MCP transport independence, H3 event-object leakage, and selected direct DB/SDK imports.

The third review found that the checker still contains loopholes/exceptions matching current violations and can produce a false green for strict dependency inversion. Plan 031B owns the final guardrail redesign after the source boundary is fixed.

## Frontend structure

- feature-specific components were grouped under `chat/`, `workspace/`, `settings/`, and `shell/`;
- `default.vue` was materially reduced to shell/data-loading composition;
- sidebar workspace dialog responsibilities were extracted;
- reusable collection/state helpers were introduced without a generic CRUD framework.

The third review considered this area materially improved. Plan 031B should verify cohesion and avoid unnecessary micro-component churn.

## Rust and deterministic acceptance

- prohibited Rust `#[cfg(test)]` modules from the Plan 031 pass were removed;
- tool-specific Rust invocation preparation became clearer while retaining one authoritative Bubblewrap/process lifecycle;
- malformed bearer rejection was moved ahead of expensive auth work;
- the Phase 4 mock IdP was updated to serve discovery + JWKS;
- the Phase 7 tool-catalog hash moved to `.agents/contracts/`;
- relevant deterministic security/contract scripts were reported passing in the previous implementation environment.

The third review found that the cheap JWT precheck became too strict by requiring `typ: JWT`, and one Rust execution comment names a helper that does not exist. Plan 031B owns those corrections.

## Dependency/gate cleanup

- the `@opentelemetry/sdk-node` manifest/lock mismatch was corrected to the compatible `0.221.x` line;
- repository policy remained no-CI and no-unit-test-suite;
- architecture checking became part of the normal commit gate.

---

# Verification status at 031A closure

The final 031A documentation recorded the following as successfully executed at/around commit `c632bc0` in the previous implementation sandbox:

- `pnpm lint`;
- architecture checker;
- Rust `cargo check` / Clippy with warnings denied;
- `pnpm audit` clean;
- `cargo audit` clean at that time;
- Phase 4 black-box acceptance;
- Phase 6 static/deterministic portion where applicable;
- Phase 7 contract acceptance;
- Phase 8 zero-bypass acceptance;
- Phase 9 SSRF script as it existed at that time.

However:

- `pnpm verify:commit` did not complete in that sandbox because Nuxt generated only the server tsconfig, not the complete client generated project required by the canonical Vue typecheck;
- `pnpm build` likewise did not complete there;
- browser/runtime verification was not completed;
- the live authenticated two-user isolation matrix was not completed;
- the third review later found that the Phase 9 redirect script did not actually exercise the redirect branch it claimed to prove.

Historical green commands must therefore not be treated as final Plan 031B evidence.

---

# Third deep-review findings transferred to Plan 031B

Plan 031B is the authoritative owner of all active findings below.

## P0

1. Provider cross-origin redirects can still forward credentials not covered by the small sensitive-header denylist, including Anthropic `x-api-key` and arbitrary secret `customHeaders`.
2. The current deterministic provider redirect test starts from an already-disallowed loopback URL, so it can pass without exercising an actual redirect hop.

## P1

3. Application contracts are still defined in infrastructure and derived from concrete implementation types.
4. `server/application/**` still imports concrete database/AI infrastructure in multiple files.
5. Repository-wide server layering is incomplete outside the thin chat route; several API routes and mixed utilities still own direct persistence/business responsibilities.
6. `server/utils/**` still hides mixed database/provider/filesystem/network ownership and can obscure transitive layer violations.
7. The architecture checker still allows type-only application → infrastructure imports, database adapters, and explicit exceptions that contradict the strict final dependency direction.
8. Rust cheap JWT prevalidation requires optional `typ: JWT`, risking rejection of otherwise valid tokens.
9. Final tenant/secret invariants must be revalidated after the architecture migration so refactoring does not reintroduce BOLA or secret leaks.
10. Full canonical commit/build/runtime verification is still outstanding.

## P2

11. Rust execution comments must match the actual inline shared process-safety path rather than naming a nonexistent `run_sandboxed` helper.
12. Project guidance/canonical memory contain stale statements about 031A closure and earlier findings.
13. Frontend/foldering should receive a final audit but should not be churned without a real cohesion/duplication issue.

For exact implementation sequence, security policy, worker lanes, acceptance scripts, architecture target, runtime matrix, and final Definition of Done, use Plan 031B only.

---

# Historical definition of done

Plan 031A's earlier detailed checklists and second-audit acceptance remain available in Git history for forensic context. They are no longer the active execution checklist.

The active completion rule is now:

> **The Plan 031 refactor family is not merge-ready until Plan 031B is complete.**

Plan 031B must not be closed merely because Plan 031A's previously checked boxes remain in history. It must independently verify final source, architecture, security, deterministic acceptance, build/type gate, and runtime behavior against its own Definition of Done.
