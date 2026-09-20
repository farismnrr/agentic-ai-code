# Plan 069 — Creative Production Platform for Games, Scenes, and Anime

Status: **CLOSED FOR CURRENT PLAN 069 SCOPE — implementation, acceptance, repository closure gates, and local commit are complete. Closure commit: `86261b9` (`fix(relay): finalize Plan 069 closure`). Push/PR/reviewed merge remain user-authorized delivery follow-up work. The closure commit itself has not been release-built/installed/restarted after commit, so do not claim the currently running relay is exactly `86261b9`; earlier Creative/media live acceptance remains valid for the behavior it exercised, while the final protected-index/toolchain hardening is source/test verified until a later operator deployment.** The 2026-09-18/19 synchronous-execution replan is implemented, built, installed, restarted, reconnected, and live-accepted. Agent execution is synchronous-only with a 60-second maximum, no `execution_mode`, no public MCP Tasks surface, `creative_job` without `wait`, and `creative_graph` without execute/rerun actions. The current source/live Blender MCP surface contains five bounded public tools (`blender_session`, `blender_inspect`, `blender_python_api_docs`, `blender_screenshot`, `blender_animation_preview`); arbitrary Python, final render, import/export, and checkpoint operations remain operator-foreground through `ai-tools creative`. TASK-052/S3, TASK-053/A6, TASK-054/G5, TASK-055 clean-room falsification, TASK-056 scoped security/failure acceptance, TASK-057 external parity, and the TASK-058 live media-consumer path now pass for the current scoped requirements. On 2026-09-20 the canonical `project_2367eb7f2843460da4b05dd1254141b8` fixture under `$HOME/Documents/Projects/Blender/plan069-final-e2e/` was restored from checkpoint `checkpoint_34bb2689e01e4253a1cca29c846e26e2`, revised in bounded operator-foreground Blender passes, and reviewed through fresh 384×216 sampled preview Assets at frames 1/31/61/91 plus a produced frame-120 screenshot artifact. The accepted minimum-quality bar is intentionally benchmark-oriented rather than commercial-anime fidelity: coherent wide→medium→close framing, readable stylized character identity/expression, visible rig deformation and motion, hair/scarf follow-through, and a readable neon-harbor stage are sufficient for Plan 069. The generic media/resource consumer path is live-proven through `creative_asset preview`: durable contained PNG Assets emit resource links plus inline image content that the connected client/model can inspect directly. No public publish, PR, or merge occurred.**

Created: 2026-09-11
Updated: 2026-09-20

**Reading / supersession rule:** this plan preserves dated implementation checkpoints as audit history. The top-level `Status`, this rule, and later explicit supersession/closure updates govern current behavior; older sections that say `current`, `open`, `partial`, quote older tool counts, or describe removed execution surfaces are historical evidence only and must not override the closed 2026-09-20 contract.

**Scope rule:** Plan 069 acceptance is visual-only. Audio, voice, music, dialogue recording, and SFX are not required Plan 069 deliverables or closure criteria. A future audio capability must be planned separately rather than inferred from Scene/Anime production terminology.

## Execution-boundary replan — 2026-09-18

Plan 069 must align with the repository-wide synchronous execution invariant introduced on 2026-09-18: agent-executed process-like work has a hard 60-second ceiling, and work reasonably expected to exceed that ceiling is handed to the human operator as an exact foreground command instead of being started, polled, detached, or hidden behind a long-lived MCP call.

Creative production therefore separates **MCP control-plane/state operations** from **operator-run heavy execution**. MCP remains the first-party contract for project/Element/Asset/job/graph state, validation, discovery, admission, lineage, bounded inspection, and cancellation. It must not expose an operation whose accepted request can legitimately spend minutes or hours executing Blender, media generation, builds, playtests, deployment pipelines, or a multi-node Creative Graph.

### Creative/Blender runtime classification

| Current surface | Plan 069 disposition | Reason |
| --- | --- | --- |
| `creative_status`, `creative_catalog` | **KEEP as MCP tools** | Pure bounded discovery/status. |
| `creative_project`, `creative_element` | **KEEP as MCP tools** | Bounded durable state CRUD/validation; no long-running executor ownership. |
| `creative_asset` metadata/history/upload-ticket actions | **KEEP as MCP tools** | Bounded registry/control-plane work. URL ingress and upload finalization must themselves remain under the public request ceiling; they must fail boundedly rather than turn into background work. |
| `creative_job compile_spec|cost_estimate|budget_status|submit|get|list|cancel` | **KEEP as MCP tools** | Admission and durable job-state control are short-lived. `submit` may create queued durable state but must not synchronously perform the heavy job. |
| `creative_job wait` as an execution trigger | **REMOVE from the public MCP execution contract** | Current implementation can execute a queued job synchronously and accepts job timeouts up to 24 hours. That directly conflicts with the 60-second agent-execution invariant. Agents inspect state with `get/list`; heavy execution is operator-run. |
| `creative_graph validate|get, template_*` | **KEEP as MCP tools** | Graph validation/state/template operations are bounded control-plane work. |
| `creative_graph execute|partial_rerun|rerun_selected|rerun_subgraph|rerun_all_dirty` when they execute nodes inline | **REMOVE from the public MCP execution contract** | A graph may contain multiple media/Blender/build nodes and legitimately exceed 60 seconds even when each node is valid. Heavy graph execution belongs to the operator CLI; MCP may store/validate the graph and later inspect resulting job state. |
| `blender_session status` | **KEEP as MCP tool** | Bounded loopback probe. |
| `blender_session start|stop` | **KEEP only with the public MCP lifecycle budget capped below 60s** | Public Blender MCP dispatch uses a 50-second internal bridge/lifecycle budget so cleanup/reap retains headroom before the repository-wide 60-second hard ceiling. Session startup shares one total deadline across the initial probe and readiness loop; it does not reset the full timeout per probe. No async/session polling workaround. |
| `blender_inspect`, `blender_python_api_docs` | **KEEP as MCP tools** | Bounded read-only inspection/knowledge. Public Blender MCP bridge transactions use one total deadline (connect + write + response share the same budget) rather than one full timeout per phase. |
| `blender_screenshot` | **KEEP only as a tightly bounded preview tool** | Public screenshot execution shares the same 50-second total bridge budget and must fail boundedly; it must not silently become a final render path. |
| `blender_execute_python` | **MOVE to operator CLI; do not expose as a public MCP tool** | Caller-authored Blender Python has unbounded algorithm/runtime behavior and host-user authority; no input-size limit can prove <=60-second completion. |
| `blender_animation_preview` | **KEEP only as the reviewed bounded sampled-preview MCP variant; use operator CLI for heavier/bulk temporal preview work** | The public tool is for bounded representative-frame review with contained PNGs and bounded inline delivery. Work that should not fit the <=60-second agent boundary remains an exact foreground operator command. |
| `blender_render` | **MOVE to operator CLI; do not expose final/animation rendering as a public MCP tool** | Even one complex still can exceed 60 seconds; animation rendering is inherently long-running. Per-frame MCP transactions avoid one bridge timeout but do not solve the agent execution boundary. |
| `blender_asset_import`, `blender_asset_export` | **MOVE heavy forms to operator CLI; retain MCP only if a strict <=60s bounded subset is proven** | Large meshes/textures/conversion/export work is scene/asset-dependent and can exceed the ceiling. |
| `blender_checkpoint_create|restore` | **MOVE heavy forms to operator CLI; retain MCP only if a strict <=60s bound is proven** | Saving/loading a large .blend file is workload-dependent. Destructive restore must never be hidden behind retry/background behavior. |
| Creative media generation/transforms, 3D generation, Game build/playtest/deploy, Scene/Anime assembly, and other binding-backed execution | **OPERATOR CLI when execution can exceed 60s** | These are production execution paths, not bounded agent RPCs. MCP owns manifests/admission/result registration and state inspection, while the human-visible foreground process owns long runtime with no Masih Awam-imposed timeout. |

### Required operator CLI path

Plan 069 must add a first-party foreground CLI surface before removing the long-running MCP actions. The exact command grammar is an implementation task, but it must support the same stable project/job/graph identities and security boundaries rather than inventing a second production model. At minimum it needs operator-visible commands for:

- executing one admitted Creative job by `project_id` + `job_id`;
- executing a validated graph/rerun from durable graph/job identity;
- Blender Python from an explicit contained script/file or otherwise reviewed operator input;
- Blender render/animation preview;
- Blender import/export/checkpoint operations that are not provably <=60 seconds;
- Game build/playtest/deploy and final Scene/Anime assembly when those routes can exceed the agent ceiling.

Operator commands are foreground commands run directly by the human/operator. **Masih Awam must not impose the agent 60-second ceiling or any platform timeout wrapper on these foreground CLI executions.** Do not wrap them in `timeout`, do not background/detach them, and do not suppress normal progress output. They may run for minutes or hours until the underlying engine/workload completes, fails, or the human interrupts it. The CLI must write results back through the same Creative Project/Asset/Job/Graph lineage so the MCP agent can inspect the completed state on the next turn.

### Public-contract acceptance rule

Before Plan 069 closure:

1. no agent-invoked MCP/tool operation may own execution that can legitimately exceed 60 seconds;
2. the relay must not advertise or expose an asynchronous MCP Tasks execution surface (`io.modelcontextprotocol/tasks`, `tasks/get`, `tasks/update`, `tasks/cancel`) as a workaround for long-running tool calls;
3. long execution must have an exact synchronous foreground operator CLI equivalent with **no Masih Awam-imposed execution timeout**, and must preserve durable job/asset/graph lineage;
4. MCP control-plane operations must fail boundedly if their own work cannot finish inside the public ceiling;
5. tests must prove that removed heavy actions and async task execution surfaces are absent, bounded retained actions still work, and operator-run completion is visible through retained MCP state/read tools;
6. a live catalog/protocol change requires rebuild/install + relay restart and then MCP client reconnect/refresh before acceptance, because connected clients may cache capabilities and `tools/list`.

This replan supersedes the earlier assumption that durable Creative jobs should be executed by a long-running `creative_job wait` RPC or that long Blender work should remain exposed merely because it is internally chunked.

## Historical implementation checkpoint — 2026-09-15 (superseded)

- Active implementation branch: `feat/plan-069-complete`, continuing from current merged `main`. Current branch plus `main` are the only active source truth for Plan 069; older feature/release-branch assumptions are history only.
- Versioning is intentionally singular: the client-visible runtime surface is built through one `runtime_tool_catalog` path and Creative state uses one active schema version (`CREATIVE_SCHEMA_VERSION = 1`). The then-observed 52-tool retained base and numbered catalog snapshots, including historical v15, are immutable history/audit artifacts rather than alternate current versions; the active retained base is defined by current source/tests, not this checkpoint. `creative_status` remains visible while disabled; the remaining Creative tools are composed into Full only when `RELAY_ENABLE_CREATIVE=true`.
- Implemented source foundation: versioned Creative Project/Element/Asset contracts, authoritative/interpreted/generated reference labels, contained pretty-JSON project state, atomic writes, candidate/accepted revision promotion, typed media metadata, SHA-256 asset provenance, source/source-surface/job/parent/Element lineage, bounded asset search, semantic capability/workflow discovery, opaque execution-binding descriptors with no default binding, persisted creative graph/job state, and the minimal reviewed control/reference DAG runtime. Reload validation now rejects unaccepted selected revisions, forged generated provenance, cyclic Asset lineage, forged manual source-surface provenance, invalid media facts/types, duplicate/invalid Scene shot ordering, invalid Game runtime paths/asset references, .
- Implemented after the initial slice: canonical contained project layout descriptors; owner-bound upload/import ingress and durable history; execution-binding discovery and explicit selection; cost/budget admission; durable job lifecycle; graph partial rerun/invalidation/concurrency/templates; Blender bridge/session/read/write/render/checkpoint/authoring surfaces; SceneBoard/Director/QA/delivery state; Anime bootstrap/character production workflows; editable Game source/build/playtest/deploy/multiplayer state; and the Nuxt Creative Canvas over the same server-owned graph contract. Generic manual asset registration still cannot forge generated/upload/URL provenance, external production bindings are never auto-selected, and public publish remains a separate authority boundary.
- Fresh benchmark audit: official Higgsfield MCP remains OAuth/client-neutral, asynchronous, credit-aware, and backed by durable Assets/history; Higgsfield may auto-select models, but Masih Awam intentionally keeps provider/model routing above the MCP boundary. Official Blender Lab MCP is released for Blender 5.1+ and explicitly warns that generated Blender Python runs without data-protection guards, reinforcing Plan 069's privileged/manual Blender-Python boundary.
- Verification completed without service mutation: `pnpm guardrail:full` passes on 2026-09-15 after the closure-fixture fixes, including repository policy, agent-doc checks, architecture, test-layout, maintainability, Rust formatting/Clippy/check/full tests, and the applicable Nuxt gates. The platform suite includes 46 passing tests; the focused security suite includes 35 passing tests; deterministic Plan 069 closure fixtures pass 5/5. The only ignored Rust test is the pre-existing operator-only real SSH client smoke requiring an explicit disposable external fixture. Verification used the repo-ignored local Rust `1.98.1` cache under `target/`; no relay/service restart, reload, real deployment, production capability activation, or public publish occurred.
- Historical live runtime handoff observed 2026-09-15: user systemd unit `ai-tools-relay.service` was restarted after installing the release binary built from `feat/plan-069-complete` at `c5f9e54768d115f355bf31a61533ba53b736e3e1`. That historical build still used separate `RELAY_ENABLE_CREATIVE=true` and `RELAY_ENABLE_BLENDER=true` activation. The current source supersedes that operator shape with the single `RELAY_ENABLE_CREATIVE` master flag; a new restart is required before the live runtime reflects the current source. Blender loopback port `9876`, Blender timeout `30000ms`, the documented credential-free `binding_local_raster` descriptor, and its `local_raster` backend mapping remain operator-owned; unrelated deployment settings and secrets were preserved/redacted. The resulting service was active/running with exit status 0 and binary version `ai-tools 0.0.14`. Authenticated `creative_status` reported Creative enabled, Blender enabled, exactly 11 Blender tools, one registered binding, schema v1, and the model/provider-agnostic flags true. A same-binary disposable Full MCP surface reported 70 unique tools (52 retained base + 7 Creative + the exact frozen 11 Blender tools) and a readable `workspace://ai-code/blender-capability`; disabled, Creative-only, and Primary probes reported 53/59/16 tools respectively with no unintended Blender exposure. The only live execution binding is deterministic `local_raster` (image capabilities); no quality-capable video/3D/deploy binding was available. Fresh live projects and receipts are recorded under disposable workspace `/home/farismnrr/Documents/Projects/plan069-live-e2e-2fN900/ai-code`: Scene `project_1e62d0f5031540a589bfa4cc4397be4b` (three shots, 10s target, Elements, board, image Assets, continuity/QA, still export, handoff); Anime `project_ba41cb34bf0e4777af200af615becb19` (accepted Character/Style, five accepted turnaround views, four accepted expressions, pose reference, 12s scene/board/QA/handoff); Game `project_dd61169986cb46bfb86895b1c418be40` (editable scaffold, accepted asset role, accepted build revision, HTTP/static smoke, iteration invalidation/rebuild, handoff); and clean-room `project_001dfefeab8748a997e4a21f9240f14a` (materially different Saltwind Observatory project with isolated IDs/paths/jobs). These live receipts are runtime evidence only; local-raster output is not production visual-quality evidence, Blender visual/character inspection is `not_inspected`, and browser interaction/console/responsive inspection is `not_inspected` because no browser provider was available.

## Workspace placement invariant — 2026-09-16

Plan 069 production/acceptance state must never use the `ai-code` source checkout as a Creative Project root. The canonical relay authorization/execution root is `$HOME/Documents/Projects`; the repository checkout is source-only and may be selected as `cwd` only while editing/testing repository code.

For Blender-backed creative work, the canonical project root is:

```text
$HOME/Documents/Projects/Blender/<creative-project>/
```

All project-internal Creative and Blender artifacts are resolved beneath that directory. The existing contained Blender layout therefore means paths such as `$HOME/Documents/Projects/Blender/<creative-project>/blender/scenes/...`, `blender/renders/...`, and `blender/exports/...`; it never means `<ai-code>/blender/...`.

Acceptance rule: any Scene/Anime/Game/Blender fixture that materializes production artifacts beneath the `ai-code` checkout is smoke/debug evidence only. It does not satisfy TASK-052/TASK-053/TASK-054/TASK-057/TASK-058 final acceptance and must be re-run from a canonical project directory outside the source repository. Historical receipts already recorded beneath disposable/copied `.../ai-code` worktrees remain historical smoke evidence only; do not copy that placement into future implementation or acceptance prompts.

## Generic media/resource review requirement — 2026-09-19

Plan 069 requires **end-to-end agent-reviewable visual artifacts**. Producing a correct file is not sufficient acceptance when the connected agent can inspect only metadata, checksums, paths, or serialized base64 text.

Producer and consumer responsibilities are distinct:

- `blender_screenshot` remains a producer that captures a bounded visual artifact.
- `blender_animation_preview` remains a producer that creates bounded sampled animation frames.
- A generic media/resource inspection layer must provide the consumer/read path for contained Creative artifacts; do not add a Blender-specific image reader.
- Native visual inspection must not require the user to manually locate and upload generated files.
- Images require a contained, MIME-allowlisted, byte-bounded, dimension-bounded inspection path that preserves asset/resource identity and provenance.
- Video/animation review must use a bounded deterministic representation such as metadata plus sampled frames/contact sheet unless a safe native client video mechanism is proven.
- Arbitrary caller-controlled absolute host paths, traversal, symlink escape, protected-path escape, and unrelated filesystem browsing are forbidden.
- Binary/base64 media bodies must never enter activity detail, activity summaries, credential redaction, logs, or telemetry.
- Existing text MCP Resources behavior must remain compatible.

The first architecture to investigate is the existing MCP Resources surface. MCP `2026-07-28` supports binary Resource contents through base64 `blob` plus `mimeType`; the pre-change Rust `ResourceContent` and Nuxt `ModernHttpMcpClient.readResource` abstractions were text-only. The current source extends those abstractions to preserve binary resource blobs, resolves reviewable image resources through durable Creative Asset identity plus selected contained project context, verifies MIME/size/dimensions/checksum before read, and has Blender screenshot/animation producers register their generated previews as Creative Assets. Tool results also emit standard MCP `resource_link` content for those image resources while retaining bounded inline-image fallback. No public generic media-read tool was added. Resource URIs remain server-derived and resolve through authorized contained Creative state rather than accepting a raw media path from the caller.

The implementation decision is evidence-driven:

1. extend generic MCP Resources if binary resource contents survive the owned server/client layers and the live ChatGPT connector exposes them natively enough for subjective image review;
2. if the live client loses native media at a boundary outside repository control, record that exact boundary and add only the smallest generic media-inspection fallback compatible with the live client;
3. do not add a new public generic tool until the Resources experiment proves it is necessary.

S3 and A6 cannot be marked complete merely because files, checksums, dimensions, or distinct temporal samples exist. Subjective visual/temporal acceptance requires media that the connected agent/model can actually inspect.

**Implementation checkpoint (2026-09-19):** source implementation for the generic image-resource path is present. Focused verification completed before the final `resource_link` wire addition: `cargo check -p ai-tools` passed; the legacy modern-client structured-output/resource test passed; all 8 focused `resources::` platform tests passed; and the focused Blender reads/preview test passed. After adding standard `resource_link` content, `git diff --check` passed, but repeated relay terminal preflight interruption prevented a fresh Cargo fmt/check/test run for that final delta. Do not treat the latest source as fully locally verified until the operator reruns the exact foreground verification commands. Live ChatGPT connector rendering remains completely unproven and is still the next acceptance boundary.

## Initial core-slice conformance audit — 2026-09-15

- **Contracts / project state:** aligned. One active Creative schema (`v1`) covers shared project, Element revision, Asset/provenance, Scene/Shot, Game, QA, graph/job identity, and production target state. Cross-track fixtures prove Scene + Anime-oriented asset/3D state + Game can coexist without provider/model/agent IDs as source of truth. Template input/output and detailed playtest-evidence contracts remain explicitly deferred to their owning later tasks.
- **Contained state / Assets:** aligned for the implemented subset. Project/graph/job JSON is contained and atomically written; manual file registration is selected-workspace-contained, protected-path-aware, SHA-256 lineaged, queryable, and cannot claim generated/upload/URL provenance. Conversation/device upload and URL import remain TASK-006 and are not emulated through arbitrary host paths.
- **Capability / workflow discovery:** aligned. Semantic capabilities and workflow descriptors are independent from an intentionally empty execution-binding inventory; pluggable nodes require a caller-selected binding and fail boundedly when omitted/unavailable. Positive multi-binding conformance remains TASK-011 rather than introducing a fake default provider/model.
- **Graph / jobs (historical 2026-09-15 slice):** the initial implementation used inline execution and an async-style submit/wait concept. That execution model is superseded by the 2026-09-18/19 boundary: public MCP owns durable graph/job validation/admission/state, while heavy graph/job execution is synchronous foreground operator work with no public wait/poll path.
- **Authority boundary:** aligned. No agent registry, prompt authoring, provider/model ranking, fallback router, arbitrary executor endpoint, raw executable graph payload, Blender host authority, or implicit deploy/publish path was added.
- **Operational boundary:** aligned. Creative remains disabled by default; `creative_status` is the activation hint; no relay restart/reload or deployment was performed during this slice.

## Goal

Build a first-party **Masih Awam Creative Production Platform** whose MCP layer provides model-agnostic and agent-agnostic creative capabilities, reusable Elements/assets, durable jobs, scene/anime/game production state, Blender/DCC execution, browser-game build/playtest primitives, QA evidence, and export/deploy boundaries **without depending on Higgsfield accounts, Higgsfield MCP, Higgsfield CLI, Higgsfield APIs, or Higgsfield-hosted generation**.

The strict interoperability benchmark is the **public Higgsfield MCP surface** documented in 2026 for the subset relevant to this product: OAuth connection, image/video/3D generation and edit utilities, reusable characters/Elements, upload/import/history reuse, durable jobs/results, quota/cost visibility, and MCP-compatible-client operation. Higgsfield's execution model may be asynchronous; Masih Awam deliberately does **not** copy that part. Masih Awam uses bounded synchronous MCP control-plane calls plus synchronous foreground operator execution for heavy work.

The architectural boundary is non-negotiable:

- **upper layer owns intelligence** — user interaction, interviews, prompt authoring, agent choice, model/provider choice, fallback policy, creative direction, and workflow/graph planning;
- **Masih Awam MCP owns capabilities and execution state** — validated semantic operations, project/Element/asset state, bounded job/graph admission and inspection, bounded Blender/DCC inspection/control primitives, deterministic/structural QA evidence, and export/deploy policy boundaries; heavy graph/media/Blender/build/playtest/deploy execution that can exceed 60 seconds is synchronous foreground operator work;
- **execution bindings are operator/runtime configuration, not product logic** — MCP may expose compatible opaque execution-binding descriptors so the upper layer can choose one, but MCP never decides that tool/capability A must use provider/model B;
- **no agent management** — Plan 069 does not add agent registries, agent spawning, role routing, skill auto-triggering, conversational interview logic, or model-selection policy to the MCP server.

Three production tracks are first-class from the architecture stage:

1. **Scene Studio** — script/brief -> Elements -> storyboard -> hero frames -> shot list -> camera/lighting -> generated or Blender-backed cinematic scene.
2. **Anime Studio** — consistent fictional characters/worlds -> turnarounds -> optional 3D/Blender assets -> rig/animation -> multi-shot anime sequence.
3. **Game Studio** — game brief -> design/STYLE FORMULA -> asset manifest -> 2D/3D assets -> playable browser build -> playtest -> optional multiplayer -> deploy, with public publication separate.

The first integrated release still advances incrementally: prove shared creative state and storyboard/scene quality first, then one short anime scene and one small verified browser game. A full anime episode, large game, or fully collaborative studio comes only after those smaller benchmarks pass.

## Success criteria

Plan 069 is successful when Masih Awam can eventually demonstrate all of the following through its own first-party contracts:

1. Any MCP-compatible upper layer can submit a fully specified creative operation or manifest and receive deterministic validation of required fields; conversational interviews and missing-information questions remain upper-layer behavior, not MCP behavior.
2. One **Creative Project** owns reusable Elements and manifests for characters, locations, props, style, scenes/shots, game assets, generated outputs, QA, and provenance.
3. The stable MCP contract is **capability-centric, not agent/model-centric**: no public tool name, project schema, workflow identity, or guidance/resource requires a particular provider/model or agent implementation.
4. Runtime capability, workflow, and compatible execution-binding discovery can report validated semantic schemas before the upper layer commits to execution; discovery never ranks or auto-selects a model/provider/agent.
5. Generation/build jobs have first-party submit/get/list/cancel/result state semantics with deterministic project ownership, bounded outputs, and retained source metadata; execution that can exceed 60 seconds is never a public wait/poll path and instead runs synchronously in the operator foreground.
6. A Canvas-style production graph can compose typed inputs, Elements, generation/edit nodes, storyboard/scene stages, Blender/DCC stages, game-build stages, QA, and delivery with partial reruns and reusable templates.
7. Reusable **Elements** provide stable project-scoped identities for Character, Location, Prop, Style, 3D Asset, Animation Clip, and approved visual-media revisions; Elements can be reused across scenes, anime shots, and games.
8. Character identity supports both explicitly authorized real-person identity-capable execution bindings and **fictional-character creation** comparable to Soul Cast: structured appearance, outfit, archetype/personality/backstory, canonical views, later rig bindings, and cross-scene consistency.
9. A Popcorn-like storyboard layer supports Auto and Manual planning modes, connected multi-frame boards, explicit reference roles, reusable Elements, and continuity of character/location/style/lighting/spatial logic before expensive video or animation work.
10. A Cinema Studio-like **Scene/Shot contract** can store and execute editable project-global style/lighting/palette rules plus per-shot camera/lens/focal/aperture/movement/framing/tempo controls; any AI Director that derives those settings from a script lives in the upper layer and submits the resulting manifest to MCP.
11. Blender remains a first-class persistent DCC backend for exact geometry, retopology, UVs, materials, hair/clothing, rigging, facial setup, animation, cameras, lighting, rendering, compositing, import/export, and checkpoints.
12. A Game Studio path can freeze a game design + STYLE FORMULA + asset manifest, generate 2D/3D assets, build while independent jobs run, verify complete gameplay locally, support single-player and reviewed local/online multiplayer paths, deploy to a playable URL, and keep marketplace/public publication separate.
13. The system can inspect visual, temporal, structural, and gameplay outputs, reject failed results, and perform narrowly scoped revisions instead of blindly regenerating/rebuilding everything.
14. The first scene benchmark produces a coherent storyboard and **10–30 second cinematic scene** from shared Elements with inspectable shot/camera/QA state.
15. The first anime benchmark produces a reusable stylized/anime character and **10–30 second anime sequence** with Blender-editable assets where the chosen workflow needs them.
16. The first game benchmark produces a small **verified playable browser game** with shared Style/Asset state, desktop/mobile input where in scope, and a contained/deployed build without silent marketplace publication.
17. Higgsfield is used only as a public product/design benchmark; no shipping code requires Higgsfield auth, tokens, binaries, endpoints, model IDs, hosted state, or services.
18. Relevant repository guardrails pass and the plan remains truthful about any engine/model/runtime capability that is not yet implemented or locally available.

## Scope

### In scope

- optional creative guidance/resources that upper layers may consume, with no MCP-owned skill auto-trigger, agent orchestration, or interview runtime;
- shared Creative Project state, **Element Library**, asset/provenance/revision manifests, and project-scoped reuse;
- semantic capability/workflow discovery plus opaque compatible execution-binding discovery;
- provider/model/agent-neutral public contracts; provider/model choice and fallback policy remain above MCP;
- first-party durable generation/build job lifecycle and reusable outputs;
- **Canvas-style typed production graphs** with branching, parallel jobs, partial reruns, templates, and eventually a visual editor surface;
- **Storyboard / SceneBoard** workflow with Auto and Manual planning modes, multi-frame continuity, reference roles, and shot manifests;
- **Scene Director** workflow with hero-frame-first planning, script-to-shot decomposition, reusable Elements, project-global look controls, and per-shot cinematography controls;
- identity/style/world state suitable for real-person references when explicitly authorized and for invented fictional/anime/game characters;
- safe attachment/materialization into production workspaces;
- Blender as a first-class live-session DCC backend and exact-production authority where editable 3D/animation is needed;
- 2D reference/turnaround -> 3D asset -> rig -> animation -> scene/shot -> render workflows;
- **Game Studio** methodology: game design, STYLE FORMULA, asset manifest, 2D/3D generation, browser build, local playtest, multiplayer path, deploy, and explicit publish gate;
- single-player, local multiplayer, and reviewed online multiplayer browser-game contracts;
- visual, temporal, structural, and gameplay QA loops;
- deterministic assembly/compositing/export/deploy where practical;
- reusable presets/templates analogous to Higgsfield Apps/Canvas recipes without copying vendor-specific implementations;
- contract/resource evaluations and scene/anime/game production acceptance fixtures;
- incremental milestones from reusable Elements and storyboards to short scenes, anime sequences, and small playable games.

### Out of scope for the first implementation

- cloning or reverse-engineering Higgsfield private backend behavior, private prompts, proprietary weights, or internal services;
- requiring Higgsfield MCP/CLI/auth/API as a runtime dependency;
- promising raw model-count parity or identical output quality for every proprietary Higgsfield model;
- one-click full-length anime episodes or large commercial games before smaller scene/game benchmarks pass;
- native console/mobile packaging in v1; the first game runtime target is a browser build unless the user provides another reviewed toolchain;
- autonomous marketplace/social/public publication without explicit user approval;
- real-time multi-user Canvas collaboration in the first milestone; design the project/graph model so collaboration can be added without rewriting asset identity;
- a generic unrestricted plugin system that executes arbitrary model-supplied workflow code;
- arbitrary remote Blender hosts in v1;
- hundreds of atomic Blender MCP tools when a smaller inspected execution surface is sufficient;
- treating generated hidden views of a character/location as factual ground truth when references do not establish them;
- silently training on copyrighted/private references beyond the user's authorized production inputs;
- making one visual style (“anime”, “game”, or “cinematic”) synonymous with one hard-coded shader, model, prompt, engine, or topology recipe;
- reproducing Higgsfield marketing/product-commerce verticals before the shared creative kernel, scene, anime, and game tracks are proven;
- MCP-owned agent registries, agent spawning, subagent routing, skill auto-triggering, chat interview orchestration, or autonomous creative planning;
- MCP-owned model/provider ranking, default-model selection, quality/cost fallback routing, or public tools whose identity hard-codes a provider/model.

## Core product decision

**Blender is not the Higgsfield replacement, generation engines are not the product, and the MCP server is not an agent/model orchestrator.** Blender is one persistent DCC backend; coding/game runtimes are another execution backend; media/AI providers are replaceable execution bindings chosen outside the MCP decision layer.

```text
UPPER LAYER — explicitly outside Plan 069 MCP ownership
User / Product UI / Agent / Skills / Orchestrator
  intent, questions, creative reasoning, prompt authoring,
  agent selection, provider/model selection, fallback policy,
  workflow/graph planning, subjective review
                      |
                      | fully specified semantic request / manifest / graph
                      v
================ MASIH AWAM MCP BOUNDARY ================
Creative Project + Element Library + Assets + Jobs
                      |
          +-----------+------------+
          |                        |
          v                        v
 Scene/Shot/Game state       Creative Graph executor
 validate/store/version      executes caller-specified DAG
          |                        |
          +-----------+------------+
                      v
             Semantic capabilities
 image | video | 3D | DCC | build | playtest | deploy
                      |
                      v
          caller-selected execution binding
         (opaque, operator-registered, validated)
                      |
      +---------------+----------------+----------------+
      |               |                |                |
 media executor                  3D executor     Blender / build runtime
      |               |                |                |
      +---------------+----------------+----------------+
                      v
        Results / deterministic evidence / lineage
                      |
                      v
             Export / Deploy / Publish Gate
=========================================================
```

The important architectural rules are:

> **Upper layers choose; MCP validates and executes.**

> **No MCP tool, workflow, Element, or project schema hard-codes a provider/model or assumes a particular agent.**

> **For pluggable executor-backed capabilities, the caller supplies `execution_binding_id`; MCP never ranks, defaults, or auto-selects—even if only one compatible binding is currently registered. Omission fails with `execution_binding_required` and compatible IDs.**

> **Elements are reusable creative identity; graphs and manifests describe production; execution bindings only execute.**

> **Scene and game production share assets/state but own different verification loops.**

An upper layer may request semantics such as `image.reference_generate`, `video.image_to_video`, `3d.image_to_mesh`, `dcc.execute`, `game.build`, `game.playtest`, or `game.deploy`, optionally with an explicit compatible `execution_binding_id`. Provider/model identifiers do not appear in the stable semantic contract, and MCP does not choose agents, models, providers, prompts, or creative direction.

## External benchmark audited on 2026-09-13

Primary public sources:

- Higgsfield skills repository: <https://github.com/higgsfield-ai/skills>
- Higgsfield skills README: <https://github.com/higgsfield-ai/skills/blob/main/README.md>
- Higgsfield install/inventory: <https://github.com/higgsfield-ai/skills/blob/main/INSTALL.md>
- Higgsfield skill-authoring guidance: <https://github.com/higgsfield-ai/skills/blob/main/CLAUDE.md>
- Higgsfield cookbook: <https://github.com/higgsfield-ai/skills/blob/main/COOKBOOK.md>
- Higgsfield CLI README: <https://github.com/higgsfield-ai/cli/blob/main/README.md>
- Generate workflow reference: <https://github.com/higgsfield-ai/skills/blob/main/higgsfield-generate/references/workflows.md>
- Generate media-input reference: <https://github.com/higgsfield-ai/skills/blob/main/higgsfield-generate/references/media-inputs.md>
- Soul ID skill: <https://github.com/higgsfield-ai/skills/blob/main/higgsfield-soul-id/SKILL.md>
- Product Photoshoot skill: <https://github.com/higgsfield-ai/skills/blob/main/higgsfield-product-photoshoot/SKILL.md>
- YouTube Thumbnail skill: <https://github.com/higgsfield-ai/skills/blob/main/higgsfield-youtube-thumbnail/SKILL.md>
- Video Explainer skill: <https://github.com/higgsfield-ai/skills/blob/main/higgsfield-video-explainer/SKILL.md>
- Game Generation skill and references: <https://github.com/higgsfield-ai/skills/tree/main/higgsfield-game-generation>
- Brandkit skill and references: <https://github.com/higgsfield-ai/skills/tree/main/higgsfield-brandkit>
- Higgsfield Canvas help/product docs: <https://higgsfield.ai/creator-hub/help-center/tools/how-do-i-use-canvas> and <https://higgsfield.ai/canvas-intro>
- Popcorn storyboard help/product docs: <https://higgsfield.ai/creator-hub/help-center/ai-models/how-do-i-use-popcorn> and <https://higgsfield.ai/storyboard-generator>
- Cinema Studio help/product docs: <https://higgsfield.ai/creator-hub/help-center/tools/how-do-i-use-cinema-studio> and current Cinema Studio 4.0 guidance
- Soul Cast product docs: <https://higgsfield.ai/soul-cast-intro>
- Higgsfield Games / Supercomputer public docs: <https://higgsfield.ai/games-intro>
- Official Higgsfield MCP landing page: <https://higgsfield.ai/mcp>
- Official MCP connection and operation guide: <https://higgsfield.ai/creator-hub/help-center/integrations/how-do-i-connect-higgsfield-to-ai-agent>
- Official MCP vs web guide: <https://higgsfield.ai/creator-hub/help-center/integrations/what-is-higgsfield-mcp>
- Official MCP/CLI distinction: <https://higgsfield.ai/creator-hub/help-center/integrations/how-do-i-access-higgsfield-via-cli>
- Official Claude/MCP creative-studio walkthrough: <https://higgsfield.ai/blog/claude-higgsfield-mcp-creative-studio>

Current public repository inventory exposes **nine skills**: generate, soul-id, product-photoshoot, brandkit, marketplace-cards, websites, video-explainer, youtube-thumbnail, and game-generation. Higgsfield's public product surface is broader than the skill inventory: Canvas provides node-based multi-model production, Popcorn provides connected storyboards, Cinema Studio provides reusable Elements plus AI-assisted directing/cinematography, Soul Cast creates fictional actors, and Supercomputer/Games owns prompt-to-playable browser-game production and deployment. Some repository guidance text still uses older skill counts or older game CLI flow; implementation-time audits must prefer the newest public inventory and product behavior rather than copying stale command names.

The benchmark is used for interoperability/design learning only. The shipping architecture below is independent.

## Public Higgsfield product-surface parity target

The goal is not pixel/UI cloning. It is **behavioral parity for the creative-production core** so a user can ask Masih Awam for the same class of work—generate media, compose workflows, direct scenes, maintain reusable characters/locations/props, create anime, build playable games, iterate, inspect, and deliver—without Higgsfield runtime dependency.

| Higgsfield public surface | Public behavior audited in 2026 | Masih Awam target | Plan priority |
| --- | --- | --- | --- |
| Skills + references | compact trigger/decision skills; detailed on-demand references; explicit route-outs/chaining | **upper-layer concern**; MCP may expose optional read-only resources but does not manage agents/skills | **outside MCP core** |
| Generate | one connector across image/video/3D, live schemas, jobs, reusable outputs | semantic capability surface + execution-binding discovery + durable jobs; upper layer chooses binding | **P0** |
| Canvas | node-based infinite production board; any model as node; branching/parallel compare; reusable workflow templates; partial execution | typed Creative Graph runtime first, then Nuxt visual graph editor; curated safe nodes; template save/reuse | **P0/P1** |
| Elements | reusable Characters, Locations, Props, saved outputs/reference media across shots/projects | project Element Library with Character/Location/Prop/Style/3D/Animation/Media types | **P0** |
| Soul ID | train/reuse a real-person identity across generation paths | optional authorized identity-capable execution bindings behind Character Elements, never the source of truth | **P1** |
| Soul Cast | construct fictional actors from structured character dimensions/backstory and reuse them across scenes | Fictional Character Builder -> Character Pack/Element -> canonical views/traits/outfit/personality/rig bindings | **P0/P1** |
| Popcorn | Auto or Manual connected storyboards; multiple references; sequence-level character/light/atmosphere/spatial consistency; frame edits | SceneBoard with Auto/Manual shot planning, multi-frame board, Element references, continuity constraints, revision lineage | **P0/P1** |
| Cinema Studio | hero-frame-first filmmaking; script-to-shot AI Director; reusable Elements; global look controls; per-shot camera/lens/focal/aperture/moves; long/reference-heavy clips | MCP stores/validates Scene/Director state and executes caller-authored shots; AI directing stays in the upper layer | **P1 state/execution** |
| Game Generation / Supercomputer Games | prompt -> game design -> asset manifest/style -> parallel asset generation + code -> local verification -> solo/multiplayer -> deploy -> optional publish | Game Studio state/build/playtest/multiplayer/deploy capabilities; design/coding intelligence stays above MCP; publication separate | **P1** |
| Websites/Apps | scaffold/edit/test/deploy full-stack product; generated media can feed app/site | reuse generic workspace/build/deploy capabilities; whichever upper layer owns coding chooses its own agent/model | **P3** |
| Brandkit / Photoshoot / Cards / Marketing | domain skills lock identity/style and compile specialist deliverables | later vertical skills built on same Element/Graph/QA kernel | **P3** |
| Virality Predictor / analysis | analyze completed media instead of generating | optional evaluator/analysis execution bindings; not required for core scene/anime/game launch | **P3** |
| Apps/effects/templates | one-click packaged workflows | saved Creative Graph templates/presets with typed inputs and bounded outputs | **P1/P2** |

**Parity claim boundary:** Plan 069 aims for near-parity in **workflow architecture and user-visible production capability** for scenes, anime, and browser games. It does not claim access to Higgsfield's proprietary models, identical visual quality, credit system, private prompts, private marketplace implementation, or exact UI.

## Higgsfield MCP parity matrix — must not be hand-waved into implementation bindings

The official MCP surface is narrower than the whole Higgsfield website but broader than generic image/video generation. Plan 069 must explicitly cover the agent-facing glue below so an external MCP client can complete the same class of workflows without hidden manual handoffs.

| Higgsfield MCP behavior | Masih Awam parity contract | Required behavior |
| --- | --- | --- |
| OAuth connection; no generation API key exposed to the agent | existing Masih Awam MCP OAuth + operator-owned execution-binding credentials | creative tools inherit authenticated MCP identity; provider secrets never enter client/model-visible args or generic terminal authority |
| One account can be connected from multiple MCP-compatible agents/clients | normal OAuth/session concurrency; no agent registry | MCP sessions are client-neutral and owner-scoped; no global “active agent” state or agent-specific runtime lock exists |
| Higgsfield requires an active subscription | **deliberate N/A** | Masih Awam MCP auth does not require a Higgsfield-like subscription; execution-binding availability/quota is operator/provider configuration and reported independently |
| Higgsfield Unlimited/free generations do not apply through MCP | **deliberate N/A with equivalent accounting transparency** | Masih Awam does not invent Higgsfield billing semantics; every selected binding reports measurable cost/quota/compute metadata honestly and budget policy applies consistently |
| All image/video generation options reachable through one connector | semantic capability discovery + `execution_binding.list/get` | MCP exposes compatible opaque execution bindings and schemas; the **upper layer** chooses the provider/model/binding. Stable tool contracts never name or auto-route to a provider/model |
| Model/tool parameters available directly through MCP | validated semantic schemas + binding-specific namespaced extensions | common capability parameters stay provider/model-neutral; an upper layer may pass a chosen binding's validated extension fields without making those fields part of the stable cross-provider contract |
| Text, reference-image, and mixed-reference generation | typed media/reference roles | one or multiple references can be attached to a job with explicit roles and selected-binding compatibility validation; parity acceptance includes at least one caller-selected image binding that can produce a 4K-class output without changing MCP schemas |
| Image upscaling | `image.upscale` | preserve lineage, requested scale/resolution, bounded output and QA |
| Video upscaling | `video.upscale` | preserve source timing and report selected-binding limitations honestly |
| Image background removal | `image.remove_background` | transparent/derived asset with parent lineage |
| Video background removal | `video.remove_background` | alpha/matte or equivalent reviewed output contract; duration/resolution bounds |
| Image expand/outpaint | `image.outpaint` | aspect/canvas expansion without overwriting accepted parent revision |
| Video generation duration + reframe/expand | `video.generate` + `video.reframe` | binding descriptors expose duration/aspect bounds; parity acceptance includes at least one caller-selected video binding capable of a >=15-second clip and target reframe without making that binding a default |
| Motion-control generation | `video.motion_control` / typed motion reference | character/reference image and motion video have distinct roles; timing/source metadata retained |
| Reusable Soul characters | Character Elements + optional identity-capable execution binding | selected character revision can be referenced by name/ID across jobs without re-uploading source photos |
| Reusable reference Elements for characters, locations, props; several per prompt | Element Library | jobs accept multiple Element IDs/revisions and preserve dependency lineage |
| Personal Clipper / long-video-to-shorts utility | `video.clip_extract` workflow | safe source ingestion, transcript/segment plan where available, selected clips with timestamp lineage; no arbitrary downloader bypass |
| Browse recent generations | `creative jobs/assets list/search/get` | query by project/type/status/source/time/Element; outputs reusable as later references |
| List prior uploads | `creative uploads/assets list/search/get` | uploaded media is first-class asset state rather than transient file paths |
| Upload a local device file through an MCP handoff | `creative upload request/complete` | external MCP clients can receive a short-lived OAuth-bound upload URL or equivalent first-party handoff; uploaded bytes land only in authorized project storage and resolve to an Asset ID |
| Import a web image/reference URL | bounded `creative import_url` | explicit HTTP(S) fetch through existing SSRF/network policy, redirect/content-type/size validation, checksum/provenance, no Blender/engine direct download |
| Reuse a past generation directly | stable Asset IDs + promoted Elements | no forced download/re-upload round trip; one accepted asset can become input to another job/graph/scene |
| Return finished media to chat while also saving it to Assets | MCP media result + Asset record | tool result includes bounded preview/resource link plus stable Asset ID; durable asset remains queryable after the turn |
| Generation history tagged by source | provenance/source field | record `mcp`, `canvas`, `scene`, `anime`, `game`, `blender`, `manual/import`, or equivalent audited origin |
| Credit balance check | `creative budget/status` | selected bindings report configured compute/quota state where measurable; paid bindings report remaining quota/cost data only when their API safely provides it |
| Cost before generation | `creative estimate` | estimate model/workflow cost or local compute class before submit; estimates carry units/source/confidence |
| “Ask before spending” user workflow | enforceable budget/approval policy | support per-job/batch/session/project thresholds and approval gates; unlike Higgsfield's prompt-only cap, hard limits should fail closed where platform policy can enforce them |
| Higgsfield generation is asynchronous and its connected agent may poll later | durable Creative job state + synchronous foreground operator execution | Public MCP exposes `submit/get/list/cancel` state only. Masih Awam intentionally does **not** copy the async/polling execution model; heavy work is one foreground operator command and results persist as stable Assets for later MCP retrieval. |
| Separate workflow catalog from model catalog | `workflow.list/get` separate from `execution_binding.list/get` | higher-level chains have schemas/cost inputs/results and create normal jobs; execution bindings are discoverable but never selected by MCP policy |
| Full multi-step production through an MCP-connected agent | caller-specified Creative Graph + Scene/Game manifests + reusable Assets/Elements | the **upper layer/agent** owns skills, planning, interviews, creative decisions, and graph construction; MCP validates/stores/admit state, while heavy graph execution is a synchronous foreground operator command without MCP-managed agents or auto-triggered skills |
| Website/App building through supported connected agents | existing generic workspace/Git/file/terminal/build/test/deploy capabilities + creative Assets/Elements | upper layer owns design/coding/model/agent choice; MCP supplies editable source/project operations and keeps build/deploy/public-publish authority distinct; no dedicated website agent is introduced |
| Browser game creation through MCP/Supercomputer surface | Game Manifest + generic workspace/build/playtest/deploy primitives + optional multiplayer binding | upper layer owns game design/code generation; MCP owns durable source/assets/build/playtest/deploy state and verification boundaries |
| Agent auto-selects a model when user does not specify one | **upper-layer responsibility; intentionally not implemented in MCP** | Higgsfield's own docs attribute this behavior to the connected agent. Masih Awam MCP exposes compatible execution bindings only; the caller supplies one for pluggable executor-backed operations. Omission returns `execution_binding_required`, never server-side selection/defaulting |
| User can force an exact model/provider in the connected experience | upper layer resolves that request to an explicit `execution_binding_id` | MCP honors the caller-selected compatible binding or rejects it precisely; it never silently substitutes or embeds provider/model names in the semantic tool contract |

### MCP parity rules

1. **Semantic capability, execution-binding, workflow, and asset/history catalogs are different concepts.** MCP does not own an agent/skill catalog. Do not collapse discovery into one ambiguous blob, and do not expose provider/model choice as a stable tool identity.
2. **Every transform is a first-class lineage operation.** Upscale, remove-background, outpaint, reframe, motion control, and clip extraction produce child assets rather than overwriting accepted inputs.
3. **External-client upload is part of the product contract.** Conversation-local attachments alone do not satisfy MCP parity because a generic external MCP client may not be able to pass local file bytes directly.
4. **Media return and media persistence are separate guarantees.** A user should see/review the result in the current conversation and still be able to find/reuse it later by Asset ID.
5. **History is queryable state, not log scraping.** Generation/upload history uses project-owned records and typed filters rather than reading raw activity logs.
6. **Cost/budget behavior is explicit.** Local-first does not mean “free”; jobs may consume GPU time, provider quota, disk, or money. MCP reports/enforces measurable bounds for the caller-selected execution binding; the upper layer decides whether that cost is acceptable and whether to ask the user.
7. **Utility/edit operations belong to the same job/QA system as generation.** They must not become ad-hoc shell commands or direct engine calls.
8. **Client-specific omissions are not platform architecture.** Masih Awam's core MCP contract should remain client-neutral and let each client expose the supported subset it can render/authorize.
9. **No agent identity is part of creative state.** Concurrent clients share owner/project state only through normal authorization and stable IDs; Plan 069 never stores an “active agent”, agent persona, model preference, or agent-specific routing state.
10. **Billing/account semantics are translated, not cloned.** Higgsfield subscription/credit rules are vendor-specific; Masih Awam parity is transparent estimate/quota/budget behavior for the caller-selected execution binding, not imitation of Higgsfield credits.

### Extended MCP parity backlog — advertised skills/post tools

The official MCP landing/blog material also demonstrates broader packaged workflows and post-production operations beyond the help-center core list. They are not blockers for the first Scene/Anime/Game milestones, but they must have an explicit home so future parity work does not invent a second platform:

| Advertised Higgsfield MCP/skill behavior | Masih Awam home | Priority |
| --- | --- | --- |
| batch/parallel campaign variants and presets | Creative Graph branches + templates + bounded batch jobs | P1/P2; core graph mechanism is P0 |
| Shorts Maker / repurpose | `video.clip_extract` + reframe + subtitle/template graph | P2 |
| motion design / animated infographics | Creative Graph + Scene Director + vector/text/layout/media nodes | P2 |
| face/character swap | reviewed identity-edit capability with explicit consent/provenance and lineage | P2, safety-gated |
| lighting/weather/background/object edits | structured image/video edit/inpaint specifications with scoped revision lineage | P2 |
| restore/stabilize/time-remap/auto-cut | post-production execution bindings under the same job/asset contract | P2 |
| color grading/reference color match | scene/style color specification + deterministic post-processing binding | P2 |
| Marketing Studio / ad multiplier | vertical skill/template over Elements + Scene/Graph; no second job/state system | P3 |
| Website Building / Apps | existing generic workspace/Git/file/build/test/deploy/publish lifecycle + creative Asset/Element imports; no managed coding agent | **MCP parity P1 contract reuse**, product templates/UI may remain later |

Any future extended utility must use the existing semantic capability/execution-binding/workflow discovery, safe ingest, Asset lineage/history, budget, jobs, QA, and graph authority. “Parity expansion” is not permission to add ad-hoc provider/model-specific public tools.

## What Higgsfield gets right — the operating model to reproduce

Higgsfield's strongest pattern is not any individual model. It separates **decision knowledge** from **execution capability**. Plan 069 preserves that separation even more strictly:

- Higgsfield-style skills/references, trigger rules, interviews, prompt strategy, model/provider choice, and creative decision trees are **upper-layer concerns** and may inspire clients/agents, but are not MCP server runtime;
- MCP exposes typed capability/workflow/execution-binding schemas instead of assuming static parameters forever;
- media has typed roles and validation before submission;
- generation/edit/build work creates durable state/results that can be retrieved later, while execution itself is synchronous foreground operator work when it can exceed 60 seconds;
- completed outputs become reusable Assets/Elements for later calls;
- deterministic/structural evidence is returned by MCP while subjective creative review stays with the upper layer unless it explicitly supplies an evaluator binding;
- revisions are lineage-preserving child operations rather than in-place mutation;
- deployment/publication is a distinct action from creation and remains explicit;
- the MCP server never manages agents or chooses a provider/model on their behalf.

Plan 069 adopts those execution/state patterns while leaving intelligence/orchestration above the MCP boundary.

## 1:1 operating-model comparison

| Higgsfield behavior | Why it matters | Masih Awam equivalent | Scene / Anime / Game use |
| --- | --- | --- | --- |
| Skill auto-trigger and `Use when` / `NOT for` boundaries | useful for connected agents, but not MCP execution authority | **upper layer only**; MCP exposes capability/resource metadata but never auto-triggers skills or manages agents | clients may route character/shot/rig/key-art work however they choose |
| Small decision-oriented `SKILL.md`, large on-demand `references/` | controls agent context cost | optional upper-layer guidance/resources; not an MCP runtime dependency | clients may progressively load anatomy/topology/toon/animation guidance without changing MCP contracts |
| Minimal, mode-specific interviews | gathers only information that changes production | **upper layer only**; MCP returns typed validation/missing-field errors | any agent/UI can ask the questions in its own style |
| Live model discovery | prevents stale executor assumptions | `execution_binding.list/get` + semantic capability discovery | upper layer sees compatible bindings/capabilities without MCP ranking or choosing providers/models |
| Separate workflow discovery from implementation catalog | treats chains as first-class products | first-party workflow registry separate from execution-binding catalog | turnaround, rigging, animation, and shot render are workflows; caller chooses compatible execution bindings |
| Media role validation | avoids malformed generation inputs | typed creative asset roles and selected-binding validation | `character_front`, `character_side`, `style_reference`, `motion_reference`, etc. |
| Auto-upload local path / reuse previous job output | lets outputs chain naturally | reviewed workspace materialization + stable asset IDs | concept art -> SceneBoard -> Blender/game asset -> shot/build |
| Canvas graph | keeps a whole multi-model workflow visible, branchable, reusable, and partially rerunnable | typed Creative Graph + saved templates + graph execution state | compare scene looks, branch anime variants, generate game assets in parallel without losing lineage |
| Elements | turns accepted characters/locations/props into reusable project assets instead of repeated prompt text | Element Library with typed references and selected revisions | same hero/location/prop reused by Scene Studio, anime shots, and Game Studio |
| Popcorn Auto/Manual storyboard | sequence consistency is solved before expensive video generation | SceneBoard Auto/Manual planning, connected frames, continuity rules, frame-level revision | direct a cinematic scene, anime board, or game cutscene with shared cast/location/style |
| Cinema Studio AI Director | script/idea becomes editable shot settings rather than an opaque monolithic generation | upper layer authors the Scene Manifest; MCP validates/stores/executes it | global style/lighting plus per-shot lens/focal/aperture/move/tempo remain engine-neutral state |
| Hero Frame First | locks composition/cast/location/look before motion makes changes expensive | approved hero frame/storyboard frame required before selected high-cost motion paths | cheaper scene/anime iteration and stronger continuity |
| Soul ID reusable identity | consistency survives many generations | Character Identity Pack | authorized real-person identity or recurring visual identity across scenes/media |
| Soul Cast fictional actor builder | invented characters need structured creation, not face training | Fictional Character Builder -> Character Element/Pack | anime/game actors with physique, outfit, traits, archetype/backstory, canonical views and later rig |
| Brandkit reusable identity system | locks visual system before assets proliferate | Style Bible + World Bible | line language, shape language, color script, shader family, environments, typography/key-art rules |
| Domain prompt enhancer | encodes specialist production language | upper layer owns creative prompt/spec authoring; selected execution binding only performs deterministic syntax translation/validation | MCP does not invent creative prompts or choose a model |
| Product Photoshoot mode router | intent chooses workflow, not surface keywords | upper-layer routing over stable MCP primitives | MCP exposes the operations/state needed for portrait, full-body, action, environment, expression, or promo work but does not classify user intent |
| Thumbnail concept gate | concept is selected before costly render | upper-layer approval policy using MCP cost estimate + assets/manifests | MCP enforces explicit submit/budget/approval boundaries but does not choose the concept |
| Style preset resolve before explainer blocks | one style key stabilizes multi-shot output | Style Element/Pack stored by MCP; upper layer selects/locks it | all shots can reference one selected revision without MCP deciding style |
| Generate narration before dependent video blocks | freezes one modality before dependent generation | caller-specified dependency DAG executed by MCP | ordering comes from the submitted graph/manifest, not MCP creative planning |
| Durable job `submit/get/list/cancel` state | generation state survives beyond one RPC without exposing async execution | first-party Creative job state + foreground operator CLI | long image/video/3D execution runs synchronously for the operator; completed state/results remain referenceable across later agent turns |
| Cost query before workflow | prevents uncontrolled spending | estimate for the caller-selected execution binding | MCP reports/enforces measurable limits; upper layer decides whether to proceed/ask user |
| Multi-variant generation with controlled dimensions | explores deliberately | variant sets with explicit changed fields | vary pose/camera/expression while identity/style remain locked |
| Post-render visual gate | output is evidence, not success by assumption | MCP returns previews/deterministic evidence; upper layer or caller-selected evaluator performs subjective review | identity, hands, costume, silhouette, line continuity, text, props, framing remain outside MCP judgment unless explicitly evaluated |
| Surgical edit from selected job | preserves good state | revision lineage + mask/scope-aware edits | change expression/background/color/camera without resetting accepted character design |
| Skill chaining via returned IDs | keeps boundaries explicit | typed project/Element/asset/job references usable by any upper-layer agent/orchestrator | no hidden conversational state or MCP-owned skill runtime is required |
| Game `STYLE FORMULA` + asset manifest | global coherence before parallel asset/code work | Style Bible + Game Design Manifest + Asset Manifest | no game code/visual batch before core loop, controls, performance budget, style, and assets are frozen |
| Parallel game asset generation + coding | uses idle generation time and treats game build as orchestration | caller-specified DAG runs independent asset jobs while any upper-layer coding system builds against stable manifest paths | faster game production without MCP managing a coding agent |
| Game local verification | a generated build is not “done” until complete loop/input/runtime errors are checked | game playtest contract + browser/runtime verification + two-session multiplayer test where applicable | verify win/lose/restart, keyboard/touch/gamepad, responsive render, fixed-step behavior, missing assets, console errors |
| Deploy separate from publish | a playable private/share URL is not the same as public marketplace publication | `game.deploy` and `game.publish` are different authority/effect boundaries | user may test/share without accidental public listing |
| Website/app create -> repo edit -> deploy | creation and publication are separate lifecycles | reuse generic workspace/build/test/deploy capabilities | any upper-layer coding system may drive them; source stays editable and deploy/publish remain explicit |
| Eval scenarios/version sync | keeps behavior testable | MCP contract/resource/graph/scene/game evals; agent-routing evals belong to whichever upper layer owns them | prove state preservation, validation, execution, QA evidence, and parity over time |

## Upper-layer mapping of the nine public Higgsfield skills — reference only

These are **behavioral examples for clients/agents above MCP**, not Masih Awam MCP-owned skills or agent routing. Plan 069 must provide the state/capabilities they would need, but the MCP server does not auto-trigger, install, select, or execute an agent skill.

| Higgsfield skill | Core mechanic to learn | Masih Awam native counterpart | Priority for target |
| --- | --- | --- | --- |
| `higgsfield-generate` | broad media work + discovery + jobs + reusable media | upper layer composes semantic media capabilities/jobs/assets | **P0 MCP primitives; routing stays above MCP** |
| `higgsfield-soul-id` | one-time reusable real-person identity training/reference | Character Element + caller-selected identity-capable execution binding | **P1** when authorized; no MCP model choice |
| `higgsfield-brandkit` | lock palette/type/logo/style, dependency-aware revisions | Style/World Elements + asset/revision contracts; upper layer authors the creative system | **P0 state primitives** |
| `higgsfield-product-photoshoot` | mode router + interview + specialist prompt enhancement | upper-layer workflow over image/edit/Element/QA primitives | **P2 upper-layer template** |
| `higgsfield-marketplace-cards` | fixed deliverable bundle from one identity | upper-layer graph/template over shared Assets/Elements | **P3 upper-layer template** |
| `higgsfield-youtube-thumbnail` | concept gate + variants + QA + surgical edits | upper-layer concept workflow over Assets/revisions/edit primitives | **P2 upper-layer template** |
| `higgsfield-game-generation` | game profile + STYLE FORMULA + asset manifest + parallel generation/build + playtest + deploy/publish split | Game Manifest/Assets/Graph/build/playtest/deploy primitives; upper layer authors game design | **P1 first-class MCP execution/state target** |
| `higgsfield-websites` | scaffold/edit/test/deploy lifecycle and media chaining | existing coding/build/deploy capabilities; whichever upper layer owns coding chooses its agent/model | **P1 for game build/deploy**, **P3 generic sites/apps** |

The first release does **not** need every marketing/business vertical or any MCP-owned skill runtime. It does need the shared mechanisms an arbitrary upper-layer agent/client requires to build equivalent experiences: Elements, graph/workflow execution, semantic capability/execution-binding discovery, durable jobs, scene/shot state, generation/edit primitives, QA evidence/revisions, editable source/project state, game playtest, and explicit deploy/publish boundaries.

## Shared creative state

Higgsfield's reusable state is split across Soul IDs, products, brand kits, presets, job IDs, and website/game resources. Masih Awam should use a more general project model.

### Creative Project Manifest

One project-level manifest identifies the authoritative production state and references all sub-manifests. It must be human-readable, diffable, versionable, and free of secrets.

Initial logical fields:

- project identity/title and production intent;
- canonical user brief and approved assumptions;
- target format/aspect/FPS/resolution where known;
- selected style pack;
- Element Library and selected revisions;
- character packs;
- world/location packs;
- asset manifest;
- SceneBoard/storyboard and scene/shot manifests;
- game design/build manifest when the project has an interactive target;
- Creative Graph/template references;
- generation/revision lineage references;
- source/provenance records;
- export/deploy/publish targets as separate lifecycle states;
- production, QA, playtest, and continuity findings.

Do not turn this into a database-first subsystem prematurely. Begin with workspace-contained structured files and promote to product persistence only when multi-session/product UX requires it.

### Element Library

Mirror the useful behavior of Cinema Studio Elements without tying identity to any one generation model. Every reusable creative object gets a stable Element ID, type, selected revision, references, provenance, and optional runtime bindings.

Initial Element types:

- `character` — Character Pack plus optional identity/rig bindings;
- `location` — World/Location Pack plus 2D/3D scene bindings;
- `prop` — canonical appearance/scale/material/use references;
- `style` — Style Pack/reference set;
- `media` — approved still/video revision promoted for reuse;
- `3d_asset` — reusable mesh/material/rig/export binding;
- `animation_clip` — approved action/motion with skeleton compatibility metadata.

Rules:

- Elements are project-scoped by default but may be deliberately imported/reused across projects through an explicit copy/reference contract.
- A generation job output is not automatically an Element; it becomes reusable only after selection/QA promotion.
- Shot, scene, anime, and game manifests reference Element IDs plus selected revisions rather than copying prompt prose.
- A revision can update one Element without silently rewriting every dependent scene/game; dependency impact must be visible.
- Element metadata never stores credentials or unrestricted engine state.

### Character Identity Pack

Generalize Higgsfield **Soul ID + Soul Cast** into one reusable Character Element/Pack. Real-person identity preparation is only one optional caller-selected execution capability; invented anime/game actors start from structured character design instead.

- authoritative reference images and their roles;
- name/ID and design notes;
- fictional-character creation fields when applicable: genre/world role, era/setting, archetype, narrative function, personality, motivation, fear, flaw, strength, backstory summary;
- face/head landmarks and distinguishing traits;
- body proportions/silhouette;
- hair construction and color rules;
- eyes/iris/pupil rules;
- skin/fur/material rules;
- canonical costume and accessory variants;
- color palette;
- expression sheet references;
- front/side/back/three-quarter turnaround references;
- 3D asset binding when created;
- armature/rig binding when created;
- facial shape-key/driver binding when created;
- execution-binding-specific optional identity artifacts such as embeddings/LoRA/checkpoints, stored as implementation details rather than product identity.

A Character Pack must be usable before model training exists. Reference-image consistency is the minimum viable path; optional training/fine-tuning can be added when measured quality justifies it.

### Style Bible / Style Pack

This is the anime equivalent of Higgsfield Brandkit plus explainer style presets:

- visual intent and references;
- allowed influence vs prohibited direct copying notes where relevant;
- shape language and silhouette rules;
- line/ink treatment;
- face/eye rendering rules;
- color palette and color-script guidance;
- value/contrast rules;
- material/toon-shader language;
- lighting language;
- background/environment treatment;
- camera/lens/composition language;
- motion/animation language;
- FX/compositing language;
- typography/key-art rules if promo assets are in scope;
- negative constraints that protect the chosen style from common drift.

The Style Pack is execution-binding-neutral. Upper layers author the creative specification; caller-selected bindings may translate validated fields into provider/runtime syntax, shader setups, or render configuration.

### World / Location Pack

Reusable environments deserve their own identity rather than being buried in prompts:

- location concept and scale;
- architecture/set dressing language;
- canonical landmarks and layout references;
- time-of-day/weather variants;
- palette/lighting relationship to the Style Pack;
- 2D environment references;
- 3D set/collection bindings when available;
- reusable camera/shot landmarks;
- continuity notes.

### Asset Manifest

Borrow the strongest game-generation pattern: **declare assets before generating a production batch**.

Each entry should identify:

- stable asset ID;
- type: character, prop, environment, clothing, hair, texture, effect, shot intermediate, etc.;
- owner pack/project;
- intended downstream use;
- required views/format/resolution;
- dependency assets;
- current source/revision;
- engine/workflow lineage;
- QA state;
- Blender binding/export when relevant.

### Scene / Shot / Sequence Manifest

Scenes are first-class shared objects for cinematic generation, anime, cutscenes, and game narrative beats. The manifest separates project/scene-global direction from per-shot controls, following the strongest public Cinema Studio pattern.

Scene-global fields:

- scene ID, story beat, goal/conflict/value shift where narrative structure is relevant;
- cast/Character Elements, Location Element, Props, Style Element;
- global genre/look, color palette, lighting logic, atmosphere, time/weather;
- target duration/FPS/aspect/resolution;
- continuity state entering/leaving the scene;
- SceneBoard/storyboard reference and approved hero frames.

Per-shot fields:

- shot ID and sequence order;
- duration/frame range;
- cast/assets/location revisions;
- staging/blocking/action/performance beats;
- shot size/composition/framing;
- camera/sensor profile when supported;
- lens/focal length/aperture/depth-of-field intent;
- camera move, movement speed, stabilization, and edit/tempo intent;
- expression/pose requirements;
- FX/lighting overrides;
- required upstream assets;
- current storyboard/hero-frame/animatic/generated-video/Blender scene/action/render references;
- continuity constraints;
- QA state.

The same scene/shot specification may compile into a generated-video execution request, Blender camera/animation setup, or a game cutscene execution path. Binding/provider/model-specific syntax does not belong in the manifest.

### Game Design / Build Manifest

Game production needs a parallel state model rather than pretending a playable game is “just another scene.” Freeze this before asset batches or implementation:

- game profile: genre, perspective, art direction, target audience, target browser/device class;
- delivery mode: design-only, assets-only, playable build, deploy, optional public publish;
- core loop and moment-to-moment verbs;
- win/lose/restart/progression rules;
- player count: solo, local multiplayer, online multiplayer;
- input contract: keyboard/mouse, touch, gamepad as applicable;
- camera/control scheme;
- world/level structure and scene relationships;
- physics/collision/timing requirements;
- fixed-step/performance budgets and asset budgets;
- language/localization requirements;
- **STYLE FORMULA** / Style Element revision used by every generated/procedural visual;
- Asset Manifest reference and stable runtime paths;
- 3D skeleton/action compatibility when animated characters are in scope;
- networking/room/state-sync contract for online multiplayer;
- build source identity and runtime/template selection;
- playtest checklist/results;
- deployment identity/URL when deployed;
- publication state separate from deployment.

### Creative Graph / Canvas Manifest

Reproduce Canvas's useful production semantics with a safe typed DAG before building a fancy visual editor.

Graph nodes may represent reviewed categories such as:

- prompt/spec input;
- Element/reference selection;
- image/video/3D generation;
- storyboard/SceneBoard generation;
- image/video edit or upscale;
- Blender/DCC operation or render job;
- game asset generation/build/playtest/deploy stage;
- QA/review gate;
- deterministic assembly/export;
- user approval/selection gate.

Graph requirements:

- typed ports and asset-role compatibility;
- immutable node/run IDs and revision lineage;
- branch/merge semantics for variants;
- parallel execution of independent nodes;
- partial rerun from a changed node without recomputing unaffected accepted ancestors;
- cache/reuse of accepted outputs where safe;
- saved templates with declared inputs/outputs;
- node status/progress and bounded failure classification;
- cost/compute estimate metadata where the caller-selected execution binding supports it;
- no arbitrary caller-supplied provider-native executable graph/plugin/script through ordinary graph nodes in the initial release;
- a future Nuxt visual editor can render/edit this same graph contract rather than inventing a second workflow model.

## Production gates

These are **state/execution preconditions**, not MCP-owned conversations. The upper layer gathers/chooses creative facts and approvals; MCP validates whether the submitted manifests/assets satisfy each gate before high-cost or high-effect execution.

### Gate A — Brief

Before generation, the upper layer should resolve and submit the facts that materially change production:

- what is being made;
- target duration/format;
- character count and which references are authoritative;
- style/reference intent;
- must-preserve details;
- caller-provided approval/autonomy metadata when the upper layer uses such a concept.

MCP does not ask these questions itself. It reports missing/invalid required fields deterministically.

### Gate B — Elements, style, and identity lock

Do not start expensive multi-asset work until:

- Style Element/Pack has an approved direction;
- each recurring main character has a Character Element/Pack with at least usable canonical references;
- recurring locations/props that matter to continuity have Elements or explicit temporary status;
- contradictions between references are recorded/resolved;
- generated missing turnaround/location views are labeled as interpretations;
- selected Element revisions are frozen for the next production batch.

### Gate C — Asset manifest

Before parallel asset generation:

- list required character/prop/environment assets;
- declare dependency relationships;
- define acceptable output/QA criteria per asset;
- choose which assets need 2D-only, 3D-only, or both.

### Gate D — SceneBoard / hero-frame / game-design lock

For scene/anime production, before high-cost motion or final animation:

- freeze scene intent and sequence order;
- produce/approve a connected storyboard or equivalent shot board;
- select representative hero frame(s) where the workflow benefits from Hero Frame First;
- freeze per-shot cast/location/prop Element revisions and camera intent;
- verify required assets/rigs exist;
- checkpoint the Blender scene before destructive scene-wide work.

For game production, before broad asset generation or gameplay implementation:

- freeze game profile/core loop/win-lose-restart/input/performance budget;
- freeze STYLE FORMULA / Style Element;
- freeze `design/assets` manifest with stable runtime roles/paths;
- resolve solo/local/online multiplayer route and required runtime modules.

### Gate E — Execute only from frozen manifests/graphs

- compile approved Scene/Game/Asset state into binding-specific execution requests or editable source without changing the stable semantic manifests;
- start independent jobs in parallel where safe;
- partial reruns must preserve unrelated accepted ancestors;
- storing caller-authored Director state does not silently trigger expensive generation/publication.

### Gate F — Visual / temporal / structural / playtest review

Never claim production completion from tool exit status alone.

- still/scene/anime work requires the applicable visual/structural/temporal review;
- game work requires local HTTP runtime verification of the complete loop, restart, missing assets, console/runtime errors, input methods, responsive render, timing/performance assumptions, and two-session multiplayer behavior when online/local-multi claims depend on it;
- failed identity/style/deformation/readability/gameplay checks trigger bounded revision rather than automatic full rebuild;
- deploy may happen only after the relevant review passes; public publish remains a separate explicit action.

## Prompt/spec compilation

Higgsfield hides specialist prompt assembly behind some domain workflows. Masih Awam should reproduce the **separation of concerns**, not a private prompt.

The **upper layer/caller** produces a structured creative specification. A selected execution binding may use a binding-specific compiler to translate that already-specified request into:

- image/video prompt text;
- reference ordering/roles;
- validated semantic capability parameters;
- namespaced binding-native extension parameters when the caller-selected binding requires them;
- Blender script/template arguments;
- deterministic compositor/layout parameters.

Rules:

1. Upper-layer client/agent logic owns creative intent, prompt authoring, workflow planning, model/provider choice, and subjective invariants.
2. MCP owns semantic request validation, durable state, job execution, and effect/approval boundaries.
3. The caller-selected execution binding may own deterministic provider syntax translation; it does not own creative model selection.
4. Raw vendor/model prompt details are not the durable project contract.
5. A revision should change only the intended structured fields when possible.
6. Binding/compiler versions must be traceable in job lineage so a result can be reproduced/explained without turning them into stable product semantics.

## Capability abstraction

Initial semantic capability vocabulary should cover at least:

```text
execution_binding.list
execution_binding.get
workflow.list
workflow.get
budget.status
cost.estimate

asset.list
asset.get
asset.search
asset.import_url
upload.request
upload.complete
upload.list

image.generate
image.reference_generate
image.edit
image.inpaint
image.control_pose
image.control_depth
image.upscale
image.remove_background
image.outpaint

video.generate
video.image_to_video
video.reference_generate
video.extend
video.reframe
video.upscale
video.remove_background
video.motion_control
video.clip_extract


3d.image_to_mesh
3d.text_to_mesh
3d.texture
3d.rig_bootstrap

storyboard.validate
storyboard.store
storyboard.revise_frame
scene.manifest_validate
scene.hero_frame_register
scene.shot_execute
scene.continuity_evidence

graph.validate
graph.execute
graph.partial_rerun
graph.template_save

dcc.inspect
dcc.execute
dcc.import
dcc.export
dcc.preview
dcc.render
dcc.checkpoint

game.manifest_validate
game.build
game.playtest
game.multiplayer_local
game.multiplayer_online
game.deploy
game.publish

project.source_inspect
project.source_mutate
project.build
project.test
project.deploy
project.publish
```

This vocabulary is a planning target, not a requirement to expose one MCP tool per line. The implementation should keep the public surface compact and allow one tool to advertise multiple semantic capabilities through discovery.

## Execution-binding strategy — model/provider agnostic by design

Do not recreate Higgsfield by replacing it with another hard-coded cloud vendor, local engine, or server-side model router.

An **execution binding** is an opaque operator-registered implementation endpoint for one or more semantic capabilities. A binding may internally represent a local runtime, one provider/model, a remote GPU worker, Blender, a build runtime, or another reviewed executor. Its implementation identity is not the public capability identity.

Every execution binding must satisfy:

- disabled by default unless explicitly registered/enabled by the operator/runtime owner;
- bounded/local or explicitly approved endpoint policy;
- no model/agent-supplied arbitrary endpoint;
- declared supported semantic capabilities and validated parameter/media-role bounds;
- bounded job concurrency/timeouts/results;
- stable project/job/asset lineage;
- credentials isolated from generic `terminal_exec` and from model-visible/client-visible arguments;
- clear effect classification for workspace writes, network use, billing/quota, host execution, or deployment;
- no claim that a local/custom executor is safe merely because it runs locally.

The MCP exposes **three discovery layers** and no ranking layer:

1. semantic capability discovery (`image.reference_generate`, `video.motion_control`, etc.);
2. workflow discovery (`workflow.list/get`) for typed higher-level executable contracts; and
3. compatible opaque execution-binding discovery (`execution_binding.list/get`) so the **upper layer** can choose an implementation.

An execution-binding descriptor exposes only bounded non-secret facts needed by the caller: stable binding ID, supported semantic capabilities, media/reference roles, parameter extensions/bounds, duration/resolution/aspect constraints, license/usage notes when known, estimate availability, and health/availability classification. Provider/model names may exist as optional operator-facing metadata, but must never be required by the stable semantic contract or encoded into tool names/workflow IDs.

**There is no MCP model-selection policy.**

- If the caller supplies a compatible `execution_binding_id`, MCP validates and uses it.
- Any semantic operation that needs a pluggable external/media executor requires a caller-selected `execution_binding_id`; even when only one binding is currently registered, MCP does not turn that accident of configuration into model-selection policy.
- If the caller omits the binding, MCP returns a bounded `execution_binding_required` result with compatible binding IDs; it does not rank, benchmark, default, or auto-select.
- Provider/model fallback, cost-vs-quality choice, agent choice, and user-preference interpretation belong entirely to the upper layer.
- Job lineage records the selected binding ID/version and a bounded implementation fingerprint for reproducibility, without making that implementation the durable project contract.

## Creative job lifecycle

Creative jobs use durable state, but **not an asynchronous agent execution API**. Public MCP owns bounded admission and state transitions only; any execution path that can legitimately exceed 60 seconds is run synchronously by the human/operator through the foreground `ai-tools creative` CLI.

```text
MCP: submit -> queued
              |
              +-> operator runs exact foreground command
                    |
                    +-> completed|failed|cancelled

MCP: get/list/cancel inspect or mutate durable state only
```

There is no public `wait` execution trigger, no Creative async task surface, no background/detached escape hatch, and no agent-side polling loop whose purpose is to bypass the 60-second execution ceiling.

A job record should return bounded metadata:

- job ID;
- project ID;
- workflow/capability ID;
- selected execution-binding ID and binding version/fingerprint;
- status/progress classification;
- creation/completion timestamps;
- safe parameters summary;
- input asset IDs;
- output asset IDs;
- failure classification without raw credential/provider leakage;
- compute/cost estimate/actual when the selected execution binding can provide it.

Raw prompts, giant workflow JSON, credentials, and unrestricted engine logs must not become routine client-visible metadata.

Jobs remain durably retrievable across normal agent turns, but durability does not imply asynchronous agent execution. A foreground operator process may run for as long as the workload legitimately requires, then write completion/failure/output lineage back into the same durable Project/Job/Asset/Graph state for later MCP inspection.

## Asset and revision lineage

Every accepted generated or imported production artifact needs a stable source relationship:

```text
source refs -> generation/revision -> output asset -> selected revision -> downstream use
```

For a surgical revision:

- select one accepted parent asset/revision;
- declare the changed field/scope;
- preserve all unrelated locked fields;
- create a new child revision rather than overwriting provenance;
- inspect the result;
- promote it only after passing the relevant gate.

This is the first-party equivalent of Higgsfield's “selected job ID -> focused edit” behavior.

## Visual QA contract

Visual QA is a product capability, not an optional chat flourish.

Anime still-image QA domains:

- character identity and silhouette consistency;
- face/eye/hair/costume continuity;
- limb/hand/anatomy defects appropriate to the chosen style;
- pose readability;
- line/shape language drift;
- palette/value drift;
- prop/logo/text correctness where relevant;
- framing/composition and focal hierarchy;
- background continuity;
- reference fidelity without pretending uncertain hidden views are authoritative.

3D/Blender QA domains:

- transform/origin/scale hygiene;
- topology and deformation readiness;
- normals;
- UV readiness;
- material/shader binding;
- hair/clothing clipping;
- rig hierarchy/constraints;
- weights and deformation at representative poses;
- facial controls/shape keys;
- camera/light/render state;
- asset naming/reuse/export readiness.

Visual QA must distinguish:

- **hard fail** — invalid output requiring retry/fix;
- **soft finding** — acceptable but improvable;
- **not inspected** — do not claim it passed when vision/preview is unavailable.

## Temporal QA contract

Anime motion must not be accepted from one still frame.

Temporal review should evaluate representative samples for:

- timing and spacing;
- silhouette clarity through motion;
- arcs and easing;
- contact/foot sliding;
- body deformation;
- facial/expression continuity;
- hair/clothing secondary motion;
- camera continuity;
- continuity between shots;
- flicker/style/identity drift in generated video;
- FX timing;

The first version may use ordered frame/contact-sheet previews before video-native MCP result types exist. It must not pretend frame sampling equals full final playback QA.

## Creative Graph / Canvas execution model

The graph manifest above is not merely storage. It is a caller-authored orchestration contract that gives Masih Awam the useful execution behavior of Higgsfield Canvas while preserving stronger boundaries. MCP validates/stores graph state and bounded metadata; graph execution/rerun that can legitimately exceed 60 seconds is synchronous foreground operator work through `ai-tools creative`. MCP does not invent the graph or choose its agents/models.

### Required graph behavior

- construct a workflow without executing it;
- validate node schemas/asset-role compatibility before execution;
- connect one node output to one or many downstream consumers;
- branch variants from one accepted Element/output;
- allow the foreground operator executor to run declared-independent branches concurrently inside one synchronous operator process;
- compare/select results and promote one revision;
- allow the foreground operator executor to rerun only the changed node and its invalidated descendants;
- retain accepted unaffected ancestors;
- save a graph as a reusable typed template;
- instantiate templates with new Characters/Locations/Props/Style without rewriting graph structure;
- show job/progress/QA state per node;
- keep generation/build/render/deploy/publish effects distinguishable;
- expose a compact MCP/tool surface even if the Nuxt UI eventually renders many node types.

### Initial safe node families

The first release should prefer a reviewed closed-world registry such as:

- `InputText`, `InputAsset`, `ElementRef`, `SelectRevision`;
- `GenerateImage`, `GenerateVideo`, `Generate3D`;
- `EditImage`, `ReframeVideo`, `UpscaleMedia` where caller-selected bindings support them;
- `StoryboardStore`, `StoryboardRevise`, `HeroFrameRef`;
- `SceneManifestValidate`, `ShotExecute`, `ContinuityEvidence`;
- `BlenderInspect`, `BlenderAuthor`, `BlenderRender`, `BlenderExport` as high-level graph stages backed by the reviewed Blender capability;
- `GameBuild`, `GamePlaytest`, `GameDeploy`;
- `VisualEvidence`, `TemporalEvidence`, `GameQA`, `ExternalReviewGate`;
- `AssembleSequence`, `ExportArtifact`.

Do not expose “arbitrary Python”, “arbitrary shell”, or arbitrary third-party custom-node graphs as ordinary Canvas nodes. Privileged underlying tools retain their own approval/effect contracts.

### Visual editor target

After the graph contract is proven headlessly, add a Nuxt visual workspace comparable in usefulness—not appearance—to Canvas:

- infinite/pannable graph board;
- typed ports and visible asset thumbnails;
- branch/compare layouts;
- node run/status/error indicators;
- reusable template save/load;
- inspect selected revision/provenance/QA from a node;
- prepare/validate selected-node/subgraph/all-dirty execution requests and show the exact foreground operator command instead of launching long-running work from the browser/MCP request path;
- no hidden direct browser access to privileged relay credentials or Blender host authority.

Real-time multi-user collaboration is later; graph/project identities must still be designed so it can be added without replacing the workflow model.

## Scene Studio — Popcorn + Cinema Studio behavioral parity

Scene Studio is the shared narrative-production layer for ordinary cinematic scenes, anime sequences, trailers/cutscenes, and story-driven game content.

### Scene intake modes — upper-layer provenance, one MCP contract

MCP does not parse a script into shots or run an AI Director. It accepts/validates SceneBoard/Shot state produced by any upper layer and records how it was authored:

- **script/brief-derived** — an upper layer parsed a paragraph/script into scenes/shots before submission;
- **auto** — an upper-layer agent/tool derived a connected board from scene intent and Elements;
- **manual** — a user/upper layer specified frames/shots explicitly;
- **existing-board** — approved frames are imported and registered into a Scene/Shot Manifest.

All modes use the same MCP schema, lineage, validation, and execution primitives.

### SceneBoard requirements

- multiple Character/Location/Prop/Style references with explicit roles;
- connected frame sequence rather than independent unrelated generations;
- configurable frame count with a conservative product bound selected at implementation time;
- shot size, action, mood, framing, and continuity intent per frame;
- consistent Element revisions, lighting logic, palette, atmosphere, and spatial relationships;
- frame-level edit/revision without discarding unrelated accepted frames;
- board can feed hero-frame generation, generative-video jobs, Blender blocking, or game cutscene production.

### Director specification boundary

Mirror the useful Cinema Studio **state contract**, not its internal intelligence. Any upper layer may act as an AI Director and submit:

- scene/shot decomposition;
- shot size and composition;
- camera/lens/focal length/aperture/depth-of-field intent;
- camera movement and movement speed;
- project-global genre/style/lighting/color rules;
- pacing/tempo and edit points;
- Element bindings;
- hero-frame choices and required references.

MCP validates required fields/references, stores/version-controls the Director specification, and estimates/enforces admission bounds. Shot generation/rendering that can exceed 60 seconds is executed synchronously by the human/operator through the foreground Creative CLI using the caller-selected binding or Blender path; MCP later inspects the durable results. MCP does **not** invent creative settings, choose an agent/model/provider, or infer an autonomy policy.

### Hero Frame First

For workflows where a still controls subsequent motion, prefer:

1. Scene/Shot intent;
2. approved Elements;
3. storyboard frame;
4. hero-frame candidate(s);
5. visual QA/selection;
6. only then motion/video/Blender animation.

This is not mandatory for every execution binding. Whether to use Hero Frame First is an upper-layer creative decision recorded in the Scene Manifest, not an MCP default chosen from model heuristics.

### Scene output levels

- board only;
- hero frames/keyframes;
- animatic/preview;
- generated video shot(s);
- Blender-backed editable shot(s);
- assembled cinematic scene;
- game cutscene package.

## Game Studio — first-class browser-game production

Game Studio is not a later “spinoff.” It is one of the three primary product tracks and reuses the same Elements, Style, Asset, Graph, 3D, Blender, QA, and coding infrastructure.

### Game intake/output modes

- **design-only** — game profile + core loop + controls + level/world design + Asset Manifest;
- **assets-only** — sprites, UI, tileable textures, environments, and rigged/animated 3D assets;
- **build/iterate** — inspect existing source or create a new browser project, preserve unchanged architecture/assets, amend manifests, build, test;
- **deploy** — produce a shareable playable deployment after QA;
- **publish** — separate explicit public marketplace/catalog action when/if Masih Awam gains such a surface.

### Full game execution workflow

The **upper layer** authors game design and chooses execution/coding agents. MCP owns the durable manifests, assets, jobs, build/playtest/deploy primitives:

1. Receive/validate a Game Design/Build Manifest containing delivery context, core loop, win/lose/restart/progression, target devices, inputs, performance budget, language, and player-count mode.
2. Store/freeze the caller-approved Style Element / STYLE FORMULA and `design/assets` manifest before broad execution.
3. Receive the caller-selected multiplayer route: solo, local same-screen, or online room/state-sync.
4. Execute independent image/3D jobs in parallel only when the submitted graph declares that independence.
5. Let any upper-layer coding system build/edit source against stable manifest paths; MCP/workspace primitives preserve source and asset identity.
6. For animated 3D assets, expose skeleton/action compatibility evidence and Blender rig/retarget primitives; the upper layer chooses the remedy.
7. Run the game over HTTP/runtime preview; never claim browser behavior from static source inspection alone.
8. Verify deterministic complete-loop, restart, assets, console/runtime errors, responsive rendering, declared inputs, timing/physics behavior, and performance budget.
9. For multiplayer, verify at least two sessions/clients and room/state synchronization behavior.
10. Deploy only after required QA/evidence gates; public publish remains separately authorized.

### Game runtime architecture

Do not hard-code the product to one frontend/game framework before implementation audit. Freeze a small reviewed browser-game template/runtime family based on actual needs:

- simple 2D/canvas/WebGL path;
- 3D web path when needed;
- platform-owned multiplayer room/state-sync module when online play is enabled;
- source remains editable in a normal workspace/repository;
- whichever upper-layer coding system is selected may use ordinary application tooling, tests, browser/runtime preview, and existing safe terminal/build surfaces; MCP does not select or manage that agent;
- generated assets are inputs to source, not opaque hosted objects that make the game impossible to continue outside a chat turn.

### Game QA and failure handling

- generation failures receive bounded retries, then the manifest is amended/compensated honestly rather than retrying forever;
- placeholder/missing assets are detected before deploy;
- game logic, render, and generated art have separate QA findings;
- a visually good build that fails restart/input/win/lose is a hard fail;
- a mechanically correct build that violates Style/Element consistency receives a visual finding;
- deployment success is not publication success and is not gameplay acceptance by itself.

## Blender Production Engine — retained and expanded from the original Plan 069

The original Blender plan remains a core implementation phase, not discarded work.

### Why Blender remains first-class

- persistent editable scene state;
- exact geometry/topology/UV/material ownership;
- rigging and deformation control;
- shape keys/drivers/facial controls;
- deterministic camera and shot composition;
- keyframes/actions/F-curves/NLA;
- hair/clothing/physics workflows;
- reusable asset libraries;
- rendering/compositing;
- checkpoints and inspectable production state.

Generative models bootstrap and accelerate production. Blender is where accepted assets become controllable production assets.

### Chosen bridge architecture

```text
Masih Awam Rust relay
        |
        | dedicated Blender tools
        | loopback TCP only
        v
official Blender Lab add-on
        |
        v
Blender / bpy
```

Do not enable generic outbound stdio MCP in Nuxt and do not spawn the official Python `blender-mcp` server as a nested subprocess. The relay implements the reviewed local bridge client directly.

### Blender security invariants

1. disabled by default;
2. loopback only in v1;
3. bounded operator-configured port/timeout;
4. no implicit Blender/add-on installation; explicit Blender process lifecycle is allowed only through `blender_session` and never through generic terminal/process arguments;
5. the Blender executable may live outside the workspace, but it must be operator-configured or resolved from a bounded reviewed set of standard executable locations/PATH entries; callers cannot supply arbitrary executable paths;
6. if an already-running compatible Blender/add-on bridge is detected, the relay attaches without claiming process ownership; `stop` may terminate only a Blender process previously launched by the relay-owned session manager;
7. raw Python is privileged host-user execution, not a sandbox;
8. read-only wrappers never accept arbitrary Python;
9. request/response bounds are relay-enforced;
10. all Blender-created or Blender-mutated production files must remain beneath the selected project workspace's dedicated `blender/` subtree; render/export/checkpoint/save destinations outside it fail closed;
11. source media elsewhere in the selected project may be materialized/copied into the dedicated Blender subtree before Blender consumes it; URLs and arbitrary host paths are never direct Blender import shortcuts;
12. activity/logging does not persist giant scripts/scene dumps;
13. attachment ingress is explicit and contained;
14. no upper-layer coding system or Plan 069 implementation step restarts the live relay/systemd service implicitly.

### Canonical Blender project layout

For Plan 069, the selected Blender creative-project root is canonically `$HOME/Documents/Projects/Blender/<creative-project>/`. Never substitute the `ai-code` source checkout as `<project>`. Blender production state is contained under one dedicated subtree beneath that creative-project root:

```text
$HOME/Documents/Projects/Blender/<creative-project>/blender/
  scenes/
  assets/
  references/
  renders/
    preview/
    final/
  animations/
  exports/
  checkpoints/
  tmp/
```

The Blender executable itself is not project data and may live in an operator-approved system/home installation path. By contrast, `.blend` files, Blender-authored assets, materialized references, previews, final renders, animation outputs, exports, checkpoints, controlled bake/cache outputs, and session temp files owned by the workflow must resolve beneath `<project>/blender/`. System/GPU/application caches that Blender or the OS owns independently are outside this production-artifact contract and are never treated as project deliverables.

### Blender v1 public MCP surface and foreground operator surface

Keep the public MCP surface compact and provably bounded:

1. `blender_session` — `status|start|stop`, retained only with a hard <=60-second lifecycle bound
2. `blender_inspect`
3. `blender_python_api_docs`
4. `blender_screenshot` — bounded preview/inspection only, never a final-render path
5. `blender_animation_preview` — bounded sampled temporal preview with contained PNG output and bounded inline image delivery; it is not a bulk/final animation renderer

The following remain first-party Blender capabilities but are **not public MCP tools** because valid workloads can exceed 60 seconds:

- `blender_execute_python`
- `blender_render`
- `blender_asset_import`
- `blender_asset_export`
- `blender_checkpoint_create`
- `blender_checkpoint_restore`

The same animation-preview capability also remains available through the foreground operator CLI for workloads that should not be attempted through the bounded public sampling surface.

Those heavy operations run synchronously through the foreground `ai-tools creative` operator CLI. There is no async Blender task API, background/detached mode, or MCP polling workaround.

`blender_session status` probes bridge compatibility without mutation. `start` launches only the reviewed operator-resolved Blender executable when no compatible bridge is already available and must fail boundedly if readiness cannot be established within the public ceiling. `stop` is owner-safe and refuses to terminate a Blender process the relay did not launch.

`blender_inspect` retains scoped structured reads for at least `scene|object|mesh|uv|rig|animation|material|nodes|physics|asset|render|character`.

Ordinary heavy authoring remains `bpy`-driven through the privileged operator execution path rather than multiplying the public MCP catalog into hundreds of atomic wrappers.

### Blender workflow guidance

On-demand guidance must cover:

- anime/stylized character proportions and silhouette;
- blockout/sculpt decisions;
- deformation-aware retopology;
- UVs/texturing;
- toon/cel materials and outlines;
- mesh/curve/Geometry Nodes hair strategies;
- modeled/skinned/simulated clothing strategies;
- skeleton/armature hierarchy;
- skinning/weight painting;
- IK/FK/constraints;
- expression shape keys/drivers;
- posing;
- keyframes/F-curves/actions/NLA;
- secondary motion/physics;
- camera/lighting;
- compositing/color finishing;
- asset naming/origins/scale/reuse/export;
- structural, visual, and temporal QA.

No guide may hard-code one anime aesthetic as universally correct.

## Anime production workflow

The target production loop is:

```text
brief
  -> style direction
  -> character identity
  -> asset manifest
  -> concept / turnaround references
  -> 3D bootstrap or manual blockout
  -> Blender cleanup / retopo / UV / materials
  -> rig / facial controls
  -> storyboard / shot manifest
  -> blocking / animation
  -> hair/clothing/secondary motion
  -> camera / lighting / toon render
  -> temporal + visual QA
  -> composite / export
  -> reusable project assets
```

### 2D-first bootstrap

For the earliest milestone, prove visual-system consistency before solving every 3D problem:

1. create Style Pack;
2. create one main Character Pack;
3. generate/author front, side, back, three-quarter, expression, and pose references;
4. inspect consistency across views;
5. mark generated hidden views as interpreted references;
6. freeze the accepted turnaround revision.

### 3D bootstrap

The **upper layer** may choose:

- image-to-3D bootstrap through a selected compatible execution binding;
- manual/scripted Blender blockout;
- hybrid workflows where generated mesh is only a starting point.

MCP validates the requested route and durable state but does not rank these options or choose one from quality heuristics. Any bootstrap execution that can exceed 60 seconds runs synchronously through the foreground operator CLI.

A generated mesh is never automatically “production ready.” It must pass Blender inspection and cleanup/retopo/UV/material/rig readiness gates.

### Rig and facial production

The first reusable character benchmark should support:

- stable skeleton hierarchy;
- practical humanoid controls;
- basic IK/FK where justified;
- usable weights;
- deformation inspection on shoulders/elbows/hips/knees;
- eye/jaw controls;
- a bounded expression set;
- checkpoint before destructive rig changes.

### Animation production

Begin with a small action, not a complete episode:

- idle/breathing;
- turn/look;
- walk/run or one action beat;
- expression transition;
- one short expressive performance beat.

Then compose these capabilities into a 10–30 second scene.

## Incremental milestone ladder

Do not skip shared foundations merely because one external model/agent can output a flashy clip or toy game. After the shared core, Scene/Anime/Game tracks may advance partly in parallel while MCP remains neutral to which upper-layer agent/model produced the requests.

### Shared milestones

| Milestone | Deliverable | Must prove before advancing |
| --- | --- | --- |
| C0 | Creative Project + manifests | Elements, assets, scenes/games, provenance, revisions are coherent/versionable and independent from any agent/model |
| C1 | Element Library + safe media ingress | accepted Character/Location/Prop/Style/Media revisions can be reused without hidden chat state |
| C2 | Capability/execution-binding/workflow/job runtime | discovery, explicit caller-selected bindings, durable jobs, result assets, cancellation, bounded effects work without hard-coded vendor/model names or MCP auto-selection |
| C3 | Creative Graph / Canvas runtime | typed DAG validates, branches, executes parallel nodes, partial-reruns, saves templates, preserves lineage |

### Scene milestones

| Milestone | Deliverable | Must prove before advancing |
| --- | --- | --- |
| S1 | SceneBoard contract | upper-layer Auto and Manual experiences submit the same connected-board schema with reusable Elements and continuity state |
| S2 | Director specification / hero frames / shot manifest | caller-authored global style/light/palette and per-shot cinematography controls are editable and execution-binding-neutral |
| S3 | 10–30 second cinematic scene | generated-video and/or Blender backend follows shot manifest, continuity, visual/temporal QA |
| S4 | reusable scene template | materially different cast/location can reuse the graph/director workflow without source-code changes |

### Anime milestones

| Milestone | Deliverable | Must prove before advancing |
| --- | --- | --- |
| A1 | consistent anime still set | one fictional character remains recognizably consistent across controlled views/expressions |
| A2 | accepted turnaround + Style Pack | authoritative/interpreted references are distinguished; asset manifest is usable |
| A3 | Blender character asset | contained import, cleanup/retopo/UV/material inspection, multi-angle visual review |
| A4 | reusable rigged character | skeleton, weights, facial controls, representative deformation QA |
| A5 | 3–5 second animation | structural animation inspection + temporal preview + render review |
| A6 | 10–30 second anime scene | SceneBoard/shot manifest, continuity, camera/lighting, final QA |
| A7 | 30–60 second multi-shot short | cross-shot continuity, reusable Elements/assets, render/composite pipeline |
| A8 | repeatable anime-production template | second project/character reuses platform without first-demo hard-coding |

### Game milestones

| Milestone | Deliverable | Must prove before advancing |
| --- | --- | --- |
| G1 | game design + STYLE FORMULA + Asset Manifest | core loop, win/lose/restart, inputs, budget, player mode, asset roles frozen before broad build |
| G2 | asset-complete local prototype | generated/procedural 2D/3D assets resolve through stable manifest paths; full game loop runs locally |
| G3 | verified browser game | keyboard/touch/gamepad claims as applicable, restart, console, responsive render, timing/performance and missing-assets checks pass |
| G4 | multiplayer game when selected | local/online route is explicit; two-session behavior and state sync pass where claimed |
| G5 | deployed playable build | shareable deployment works; source/project state remains editable; public publish has not happened implicitly |
| G6 | repeatable game template | materially different genre/style can reuse Game Studio contracts without hard-coded first-game assumptions |

Full anime episodes, large games, broad public marketplaces, and real-time collaborative studio workflows are explicitly **post-benchmark expansion**, not prerequisites for proving the platform kernel.

## Upper-layer guidance compatibility — explicitly outside MCP ownership

Higgsfield publishes skills because connected agents need production knowledge. Masih Awam can provide equally useful **optional resources, example graphs, schema docs, and production guides**, but Plan 069 does not create or manage an agent/skill runtime.

The boundary is:

- MCP exposes typed capabilities, schemas, Elements/Assets, manifests, jobs, graph execution, Blender/build/playtest/deploy primitives, and bounded resources/documentation;
- any upper layer may package those primitives as skills, system prompts, UI flows, automation recipes, or human workflows;
- upper layers own trigger phrases, route-outs, interviews, creative reasoning, prompt enhancement, agent/subagent delegation, model/provider choice, fallback policy, and subjective approval;
- MCP resources must not contain executable authority or silently trigger jobs;
- all chaining uses typed project/Element/Asset/job/graph IDs rather than hidden conversational state;
- provider/model names may appear only in optional execution-binding metadata or upper-layer documentation, never as required MCP tool/workflow identities;
- changing the upper-layer agent or skill framework must not require a change to Scene/Anime/Game MCP contracts.

### Optional resource families

Plan 069 may expose concise, progressive resources for upper layers, for example:

- semantic capability and media-role guidance;
- Character/Style/World/Scene/Game manifest examples;
- Blender production references;
- SceneBoard/Director schema examples;
- game build/playtest checklists;
- Creative Graph examples/templates;
- QA evidence interpretation guides;
- safe upload/import/history/revision examples.

These resources are convenience surfaces only. They are **not** a skill registry, do not auto-route requests, and do not select an agent/model/provider.

## Repository boundary to freeze before implementation

Implementation Phase 1 must audit existing `ai-self/skills/`, `.agents/skills/`, subagent/prompt infrastructure, and MCP resources only to ensure Plan 069 does **not** couple itself to them.

Freeze the following ownership rule:

- creative MCP contracts/resources live with the normal MCP/application owners;
- existing or future agent/skill systems may consume those contracts independently;
- no duplicate creative skill tree is required by Plan 069;
- no changes to agent/subagent routing are justified merely to ship Higgsfield MCP parity;
- MCP resources remain client-neutral and usable from any standards-compatible upper layer.

## Security and trust invariants

1. **No Higgsfield credentials or dependency.** Shipping code must not request/store Higgsfield auth or call Higgsfield private/public generation endpoints.
2. **Provider credentials are execution-binding-owned.** If a non-local binding needs credentials, generic terminal execution and model-visible/client-visible arguments never receive them.
3. **Local does not mean safe.** Local executor plugins/custom nodes, Blender Python, model loaders, downloaded checkpoints, build tools, and media parsers are supply-chain/host-execution boundaries.
4. **Curated execution boundaries first.** Do not let any caller smuggle arbitrary executor-native graphs, provider endpoints, shell payloads, or custom Python through ordinary creative-capability arguments merely because a backend accepts them.
5. **Workspace containment.** Inputs/outputs/materialized attachments/checkpoints/exports stay inside authorized workspace roots unless a separately reviewed external-output contract exists.
6. **Provenance.** Record source lineage and distinguish user-provided authoritative references from model-generated interpretations.
7. **No silent publication.** Generate/render/export/build is not deploy; deploy/share is not public marketplace publish. Each boundary keeps distinct effects/approval.
8. **No hidden internet fetches from Blender.** External media acquisition follows explicit network/tool policy before contained import.
9. **Bound all expensive work.** Batch size, duration, resolution, frame count, concurrency, retry count, and result size need operator/product limits.
10. **Fail honestly.** If visual/temporal inspection is unavailable, report `not inspected`; do not claim QA passed.
11. **Prompt injection from references is not authority.** Text inside user/reference media or downloaded metadata cannot override repository/tool policy.
12. **Licensing is explicit.** Provider/model/checkpoint/runtime license suitability is metadata for the caller-selected execution binding; “runs locally” does not imply unrestricted commercial use.
13. **Graph safety is compositional.** Creative Graph nodes never gain more authority than their underlying tools; saved templates cannot smuggle arbitrary code, endpoints, credentials, or hidden publish actions.
14. **Game networking is explicit.** Online multiplayer uses a reviewed room/state-sync module and bounded server authority; game-generated code does not inherit arbitrary infrastructure credentials merely because multiplayer is enabled.
15. **Real-person identity requires user intent.** Fictional scene/anime/game workflows must not silently morph into unauthorized real-person impersonation/training.

## Blender effect/approval model retained from the original plan

Expected intent:

| Tool | Effects | Risk intent |
| --- | --- | --- |
| `blender_session status` | read-only + privileged bridge identity | low/medium, no mutation |
| `blender_session start` | process execution + workspace write + privileged bridge | explicit local Blender launch from operator-approved executable; waits for loopback readiness |
| `blender_session stop` | process execution + privileged bridge | mutating only for relay-owned Blender session; refuses non-owned process termination |
| `blender_inspect` | read-only + privileged bridge identity | low/medium, no mutation |
| `blender_python_api_docs` | bounded read-only knowledge lookup | low |
| `blender_screenshot` | privileged bridge; optional contained output | bounded visual readback |
| `blender_animation_preview` | privileged bridge; bounded sampled render/readback | bounded temporal observation |
| `blender_render` | workspace write + compute + privileged bridge | mutating, explicit output scope |
| `blender_asset_import` | workspace read + scene mutation | contained ingress |
| `blender_asset_export` | workspace write + privileged bridge | contained egress |
| `blender_checkpoint_create` | workspace write + privileged bridge | controlled recovery write |
| `blender_checkpoint_restore` | destructive scene replacement | high/manual approval |
| `blender_execute_python` | privileged bridge + open-world host execution | high/manual approval |

Creative execution tools need effect review based on the caller-selected execution binding: local/remote, workspace writes, network/billing, host execution, deployment, and arbitrary-code risk remain explicit regardless of provider/model.

## Canonical implementation baseline

The single active source baseline for Plan 069 was re-audited from `origin/main` on 2026-09-14:

```text
ae9571448a7c92e9ecf6f73b51b05009adc310e7
```

Implementation proceeds only from this reconciled baseline and the current `feat/plan-069-creative-core` branch. Earlier SHAs, release branches, and numbered tool-catalog snapshots are historical audit artifacts only; they are not alternate active implementations or contract versions.

Phase-01 audit confirmed that no prior optional-capability prototype defines current `main` behavior. Historical branches do not define the active contract and no source is ported wholesale from them. The current implementation owns one runtime catalog composition path and one active Creative schema version (`CREATIVE_SCHEMA_VERSION = 1`).

## Expected implementation ownership

Exact new module names are frozen only after the Phase 1 architecture audit, but ownership should follow existing repository layers:

- `packages/rust-tools/src/core/config/` — operator capability config and bounds;
- `packages/rust-tools/src/application/` — first-party runtime/job/execution-binding/Blender application logic;
- `packages/rust-tools/src/interfaces/mcp/` — compact MCP tool schemas and capability metadata;
- `packages/rust-tools/src/application/resources.rs` — bounded resources/capability guidance when Plan 069 needs a read-only resource surface;
- `packages/rust-tools/tests/` — Rust integration/security/contract tests;
- `server/application/` / `server/infrastructure/` only when product persistence, attachment materialization, or shared policy requires Nuxt ownership;
- `shared/` only for genuine cross-client/product contracts;
- optional resources/guidance may live in the repository for upper-layer consumers, but Plan 069 does not establish an MCP-owned skill/agent runtime;
- `.agents/knowledge/`, canonical memory, operator docs — durable architecture/setup/security guidance.

Do not put a generic arbitrary media-engine process spawner into Nitro and do not reopen stored stdio MCP execution as a shortcut.

## Phase overview

| Phase | Goal | Depends on | Exit criteria |
| --- | --- | --- | --- |
| PHASE-01 | Reconcile baseline and freeze architecture/contracts | none | upper-layer/MCP boundary, Elements/state, execution-binding/capability/job/workflow/graph, scene/game and Blender contracts frozen |
| PHASE-02 | Creative Project + Element state + safe media ingress | PHASE-01 | C0/C1 project/Element/materialization contracts proven |
| PHASE-03 | Capability/workflow registry + creative jobs + graph contract + minimal headless executor | PHASE-01 | C2 contracts work without hard-coded model names; representative graphs validate and execute through the minimal reviewed DAG runtime |
| PHASE-04 | Initial reference-aware media generation | PHASE-02, PHASE-03 | Character/Style still consistency foundation passes A1/A2-quality gate |
| PHASE-05 | Identity/style/world/Element production system | PHASE-04 | reusable Characters/Locations/Props/Style Elements drive revisions and reuse |
| PHASE-06 | Blender first-class production engine | PHASE-02, PHASE-03 | five bounded public Blender MCP tools pass, including sampled temporal preview; heavy Blender capabilities remain synchronous foreground operator commands; cold start/readiness/owner-safe stop and project-contained outputs remain proven |
| PHASE-07 | Anime 3D character asset pipeline | PHASE-05, PHASE-06 | A3/A4 pass |
| PHASE-08 | Animation, facial, and temporal QA | PHASE-07 | A5 passes |
| PHASE-09 | Scene Studio: SceneBoard + Director + shot orchestration | PHASE-04, PHASE-05; PHASE-06 optional per backend | S1/S2 pass and one S3 candidate can be produced/revised |
| PHASE-10 | Unified visual/temporal/structural QA + surgical revisions | PHASE-04, PHASE-08, PHASE-09 | failed scene/anime/media outputs are detected/revised with lineage |
| PHASE-11 | Sequence assembly/export + anime delivery | PHASE-08, PHASE-09, PHASE-10 | A6 plus contained reusable project delivery passes |
| PHASE-12 | Game Studio: design/assets/build/playtest | PHASE-02, PHASE-03, PHASE-05 | G1/G2/G3 pass for a small browser game |
| PHASE-13 | Game multiplayer/deploy + source-preserving iteration | PHASE-12 | selected G4 path passes where applicable and G5 deploy contract passes |
| PHASE-14 | Mature Creative Graph runtime + Canvas-style workspace + MCP parity/resource evals | PHASE-03 and proven scene/game workflows | C3 production graph runtime/templates/UI and high-value Higgsfield MCP operating parity pass |
| PHASE-15 | Triad acceptance and closeout | all required prior phases | fresh Scene S3, Anime A6, and Game G5 benchmarks pass; second-project falsification and repository gates pass |

# PHASE-01 — Reconcile baseline and freeze contracts

**Goal:** turn the benchmark into a repository-native architecture before implementation starts.

**Dependencies:** none.

### TASK-001 — Re-audit current runtime baseline

**Outcome:** implementation starts from the actual current task/resource/catalog architecture, not from optional-capability assumptions or historical release snapshots.

**Files:** read current `packages/rust-tools/src/`, `server/infrastructure/mcp/`, shared capability policy, and current contracts after updating from `main`.

**Steps:**

- [x] Verify repository identity, clean task branch/worktree, and current `main` HEAD.
- [x] Confirm which previously assumed optional-capability components actually landed. *(Reconciled result: none of the assumed creative/optional framework existed on the active baseline, so current source is authoritative.)*
- [x] Re-audit current MCP protocol/tool/resource/task contract.
- [x] Re-audit attachment/file ingress and image result support. *(No reusable first-party creative attachment/upload materialization path was found; TASK-006 remains open.)*
- [x] Re-audit Blender Lab bridge/version/security docs.
- [x] Re-audit the official Higgsfield MCP help pages/landing page operation list first; use public skills only as upper-layer/product reference, never as MCP authority.

**Validation:** architecture findings are recorded in this plan or durable knowledge without stale release-branch assumptions.

**Commit boundary:** `docs(plan): reconcile creative platform baseline` only if durable docs materially change.

### TASK-002 — Freeze the upper-layer / MCP responsibility boundary

**Outcome:** Plan 069 cannot accidentally turn the Masih Awam MCP server into an agent framework or model router.

**Files:** MCP catalog/resources, capability policy, creative contracts, and any existing prompt/subagent/skill integration only as boundary-audit inputs.

**Steps:**

- [x] Document that agents, subagents, skill auto-triggering, interviews, creative prompt authoring, workflow planning, provider/model ranking, and fallback policy are outside Plan 069 MCP ownership.
- [x] Define the semantic request envelope accepted from any upper layer.
- [x] Define opaque `execution_binding_id` discovery/validation without provider/model-specific public tool names.
- [x] Define the missing-selection/ambiguity error returned when a pluggable capability has no caller-selected binding.
- [x] Define optional MCP resources/guidance as read-only reference material with no authority to auto-run or route an agent.
- [x] Verify existing agent/subagent infrastructure is not extended merely to implement creative MCP parity.

**Validation:** architecture/contract tests can prove that two different upper-layer clients/agents can call the same creative MCP capability with different selected execution bindings and no MCP-owned agent/model-selection path is invoked.

**Commit boundary:** `docs(creative): freeze agnostic mcp boundary` or the smallest contract commit required during implementation.

### TASK-003 — Freeze creative project schemas

**Outcome:** versioned Creative Project, Element, character, style, world/location, asset, scene/shot, game, graph identity, QA finding, and lineage contracts are specified before engine integration. Template input/output and detailed playtest-evidence contracts remain owned by TASK-010 and the Game Studio phases rather than being implied complete here.

**Steps:**

- [x] Define required vs optional fields.
- [x] Define stable IDs and relative workspace references.
- [x] Define authoritative vs interpreted/generated reference labels.
- [x] Define revision lineage.
- [x] Define forward-compatible schema versioning.
- [x] Define bounds and secret-exclusion rules.

**Validation:** `packages/rust-tools/tests/platform/creative/contracts.rs` now exercises one provider-neutral cross-track project containing Character/Location/Prop/Style/3D Elements, a three-shot cinematic Scene, anime-oriented asset lineage, a browser-game manifest, QA state, and graph/job identities; persisted-state falsification covers unaccepted selected revisions, forged generated authority, path traversal, cyclic Asset lineage, and forged source-surface provenance.

**Commit boundary:** `feat(creative): add production manifest contracts`.

### TASK-004 — Freeze MCP-facing capability, execution-binding, workflow, budget, asset, and job contracts

**Outcome:** semantic capabilities, opaque execution-binding discovery, workflow discovery, budget/cost preflight, asset/history access, job lifecycle, effect policy, and result-media delivery are specified independently from agents/providers/models.

**Steps:**

- [x] Freeze semantic capability vocabulary needed through C3/S3/A6/G5 plus MCP utility parity: upscale, background removal, outpaint/reframe, motion control, and clip extraction.
- [x] Freeze capability list/get representation for durable upper-layer discovery.
- [x] Freeze opaque execution-binding list/get representation separately from semantic capabilities; provider/model names are optional implementation metadata, never stable contract identity.
- [x] Freeze workflow list/get separately from execution-binding inventory.
- [x] Freeze caller-selected-binding semantics and explicit missing-selection failure; MCP has no automatic model/provider selection or fallback policy.
- [x] Freeze Asset/history `list/get/search` filters, source/source-surface tags, stable IDs, typed media metadata, acceptance state, and lineage. *(Upload-specific records and current-turn preview/resource delivery remain open under TASK-006 and the result-delivery item below.)*
- [x] Freeze secure external-client upload request/complete and bounded URL-import contracts.
- [x] Freeze cost/compute estimate plus global/project/session/job budget-status and hard-limit semantics.
- [x] Freeze public job submit/get/list/cancel behavior and cross-turn retrieval; no public wait/poll execution trigger exists, and heavy execution is synchronous foreground operator work.
- [x] Freeze result and failure bounds, including current-turn preview/resource delivery plus durable Asset registration.
- [x] Freeze external-cost/network vs local-compute effect classifications.
- [x] Decide whether existing relay Tasks can own creative long-running jobs directly or need a thin creative domain layer over the same manager. *(Decision: current process-only/in-memory manager is not forced to own creative state; use a thin project-persisted creative domain layer and preserve a replaceable lifecycle boundary.)*

**Validation:** mock bindings can advertise different implementations for the same semantic capability without changing MCP contracts; the same fixture can discover compatible binding IDs, require the upper layer to select one when ambiguous, preflight cost for that binding, submit, list/retrieve the job, and reuse the returned Asset ID.

**Commit boundary:** `feat(creative): define capability and job contracts`.

**Phase exit criteria:**

- [x] no implementation depends on a Higgsfield contract;
- [x] the upper-layer/MCP boundary is explicit and no agent/model router was added;
- [x] creative state schemas are frozen for initial milestones;
- [x] execution-binding/job semantics reuse existing platform primitives where possible;
- [x] Blender remains a separate privileged DCC capability under the master architecture.

# PHASE-02 — Creative Project, Element state, and safe media ingress

**Goal:** make references and generated outputs durable, contained production assets.

**Dependencies:** PHASE-01.

### TASK-005 — Implement contained creative project workspace layout

**Outcome:** each project has bounded manifests plus Element, asset/revision, SceneBoard, graph/template, and game-build state without escaping authorized workspaces. Production placement is separate from repository source: the relay root is `$HOME/Documents/Projects`, and Blender-backed project work is rooted at `$HOME/Documents/Projects/Blender/<creative-project>/`, never at the `ai-code` checkout.

**Steps:**

- [x] Define contained path layout for project manifest, Elements, assets/revisions, scene/shot boards, creative graphs/templates, game design/build metadata, QA/playtest evidence, and exports.
- [x] Make Plan 069 production/bootstrap/acceptance flows select or create the canonical project root outside the `ai-code` source checkout; Creative project admission rejects the `ai-code` source checkout, Blender tool admission requires a `.../Blender/<creative-project>` root, Blender-backed acceptance uses `$HOME/Documents/Projects/Blender/<creative-project>/`, and repo-local creative outputs remain smoke-only.
- [x] Ensure paths remain relative/canonical and cannot target protected credentials.
- [x] Add atomic manifest updates.
- [x] Preserve human-readable diffs.
- [x] Bound manifest and metadata sizes.

**Validation:** traversal/symlink/protected-path tests fail closed; normal project fixture round-trips.

**Implementation update (2026-09-16):** source admission now prevents Creative production state from using the `ai-code` checkout as its project root and requires every Blender tool call to resolve from a canonical `.../Blender/<creative-project>` directory. Deterministic tests cover both rejection rules, idempotent repeated Blender materialization, bootstrap-object selection, actual rig weighting/fallback evidence, and Blender 5.2 action-channel compatibility. The pre-restart platform suite passes 52/52 and `pnpm guardrail:full` passes with the expected single operator-only SSH fixture ignored. Live production acceptance must still be rerun after installing/restarting the new binary from `$HOME/Documents/Projects/Blender/<creative-project>/` before Plan 069 closure.

**Commit boundary:** `feat(creative): add contained project state`.

### TASK-006 — Implement MCP-safe media ingest: conversation attachment, device upload handoff, and URL import

**Outcome:** media from a Masih Awam conversation, an external MCP client's local device, a reviewed web URL, or a prior generated Asset can become a safe stable project Asset without arbitrary host-path assumptions.

**Steps:**

- [x] Reuse an existing reviewed first-party conversation attachment handoff if present. *(Audit found none on current `main`; the new first-party handoff below is therefore the owning path.)*
- [x] Otherwise add the smallest owning-layer attachment materialization primitive. *(Conversation/device bytes use the same reviewed ticketed byte-ingress and contained binary writer; no arbitrary host path is introduced.)*
- [x] Add an external-client upload request/complete flow that returns an expiring OAuth-bound upload URL or equivalent reviewed first-party handoff rather than requesting arbitrary local filesystem paths from the agent.
- [x] Bind upload requests to owner/project, media class, size limit, expiry, single-use/replay policy, and final Asset ID.
- [x] Add bounded HTTP(S) URL import only through the existing SSRF/network policy: redirects, DNS/IP class, content type, size, timeout, filename and checksum/provenance are validated before persistence.
- [x] Validate media type, size, destination ownership, filename/path, and source metadata for every ingest route.
- [x] Never accept arbitrary host destination paths or let Blender/media engines fetch URLs directly as an ingest shortcut.
- [x] Preserve source/provenance and source-mode (`conversation_upload`, `mcp_upload`, `url_import`, `generated_asset`, etc.).
- [x] Keep model-visible attachment access distinct from Blender/engine-readable workspace files.

**Validation:** conversation-upload, external-upload-handoff, URL-import, and prior-Asset fixtures materialize to stable Asset IDs; expired/replayed upload tickets, private-network URL targets, redirect escapes, oversized/unsupported media, path injection, and owner/project mismatch fail closed.

**Commit boundary:** `feat(workspace): materialize creative attachments safely`.

### TASK-007 — Implement queryable asset/upload/history registry and revision lineage

**Outcome:** imported/generated/revised assets get stable project IDs, parent/child lineage, queryable history, and MCP-safe current-turn media delivery.

**Steps:**

- [x] Register source type, source surface (`mcp|canvas|scene|anime|game|blender|manual/import` or equivalent), media role, project, Element binding, and accepted/candidate selection state.
- [x] Record bounded output metadata/checksum/dimensions/duration where appropriate.
- [x] Link originating creative job and parent Asset lineage.
- [x] Mark accepted vs candidate revisions.
- [x] Implement bounded list/get/search filters for media type/role, job lineage, Element, project, source/source-surface, acceptance state, parent Asset, and time range. *(Upload-specific filters remain meaningful once TASK-006 adds upload records.)*
- [x] Return reusable Asset IDs plus bounded preview/resource metadata so a result can be reviewed in the current client and reused later without download/re-upload. *(Current MCP result returns stable Asset identity, media facts, relative contained path, checksum, and byte bounds; richer client resource rendering may layer on this contract later.)*
- [x] Keep raw credentials/prompts/unrestricted provider logs out of routine metadata. *(Asset metadata is a typed media-facts object; arbitrary extra keys are rejected.)*

**Validation:** fixtures can (a) trace user reference -> generated turnaround -> revised selected image -> Blender import, (b) list recent generated/uploaded assets with stable filters, and (c) reuse one previous Asset directly as a later job reference.

**Commit boundary:** `feat(creative): track asset revision lineage`.

**Phase exit criteria:**

- [x] production media can be safely materialized;
- [x] every asset has bounded provenance/lineage;
- [x] no Blender/generator path assumes transient chat paths are filesystem paths.

# PHASE-03 — Capability/workflow registry, creative jobs, and minimal graph runtime

**Goal:** reproduce Higgsfield's live discovery/job strengths without vendor coupling, and establish the smallest reviewed headless DAG executor that downstream Scene/Anime/Game phases can reuse instead of inventing temporary orchestration.

**Dependencies:** PHASE-01.

### TASK-008 — Add semantic capability, execution-binding, and workflow discovery

**Outcome:** any client/agent can discover durable semantic capabilities, compatible opaque execution bindings, and workflows with validated schemas without MCP ranking/selecting a provider/model.

**Steps:**

- [x] Add operator-disabled-by-default creative capability group.
- [x] Expose active semantic capabilities independently from execution implementations.
- [x] Expose bounded execution-binding list/get descriptors: opaque binding ID, capabilities, reference roles, validated extension schema/bounds, duration/resolution/aspect constraints, availability/health, known license notes, and estimator support. *(The implementation inventory is intentionally empty until a reviewed binding is registered.)*
- [x] Expose workflow IDs and workflow list/get separately from execution bindings.
- [x] Never rank/default/auto-select a binding. For pluggable executor-backed operations, omission returns `execution_binding_required` with compatible binding IDs regardless of how many bindings are currently registered.
- [x] Validate an explicit caller-selected binding and return precise incompatibility/unavailable diagnostics instead of silent substitution.
- [x] Keep endpoint/credential/raw engine implementation metadata hidden; provider/model display metadata, if exposed at all, is informational and non-contractual.
- [x] Return activation/setup hint when disabled.

**Validation:** this core slice proves stable semantic capability/workflow discovery, a truthful empty execution-binding inventory, mandatory `execution_binding_required` on pluggable nodes with no selection, bounded `execution_binding_unavailable` for an unknown explicit binding, and no provider/model auto-routing. Positive two-binding conformance is intentionally deferred to TASK-011, where reviewed mock/real bindings actually exist; this task does not invent fake default executors merely to satisfy discovery tests.

**Commit boundary:** `feat(creative): expose capability discovery`.

### TASK-009 — Add first-party creative job lifecycle, cost preflight, and enforceable budgets

**Outcome:** generation/transform work can be estimated and admitted through bounded MCP state, then executed synchronously in the operator foreground when it can exceed 60 seconds; jobs remain listable/cancellable/retrievable with stable project ownership and bounded spend/compute authority. No async/poll/wait execution API is part of the public contract.

**Steps:**

- [x] Reuse existing job/task lifecycle semantics and cancellation where possible; Creative jobs remain durably project-owned because the process JobManager is intentionally in-memory/process-oriented and cannot provide cross-turn creative retrieval.
- [x] Add `cost.estimate`/compute-estimate semantics before submit when the selected execution binding/workflow can provide meaningful bounded estimate data.
- [x] Add `budget.status` for measurable operator limits: compute approval/hard limits, project admitted/remaining compute, output bounds, concurrency, retries, and explicit `binding_owned_not_reported` provider quota when the binding cannot expose credits safely.
- [x] Enforce operator-configured thresholds before expensive jobs/batches; approval can cross only the soft approval threshold and cannot override job/project/output hard maxima.
- [x] Add domain metadata only where creative workflows need it: owner, graph/capability/workflow target, binding, estimate, retry/timeout, output lineage, and bounded failure code.
- [x] Support public submit/get/list/cancel and cross-turn retrieval through durable project job records; heavy execution/wait semantics are operator-CLI-only and synchronous foreground work.
- [x] Bound concurrent jobs/batches and retry count.
- [x] Persist/recover only if current platform task semantics cannot satisfy cross-turn retrieval safely. *(Current task state is in-memory/process-oriented, so creative graph/job records are project-persisted.)*
- [x] Keep failure diagnostics bounded/classified; provider/cost diagnostics remain absent until bindings/cost estimation exist.

**Validation:** the non-default debug/test-only `test-creative-binding` conformance path covers queued/running/completed/failed/cancelled, list/retrieve after a fresh call, estimate-before-submit, soft approval, job/project/output hard-budget denial, execution timeout, actual-output overflow, retry bounds, and owner isolation. Production/release builds do not gain this executor; unimplemented real bindings still fail `execution_not_implemented` until their owning phase supplies execution.

**Commit boundary:** `feat(creative): add generation job lifecycle`.

### TASK-010 — Add curated workflow registry, Creative Graph contract, and minimal headless executor

**Outcome:** multi-step workflows are discoverable independently from execution bindings, and a typed graph can represent and execute caller-specified composition through one minimal reviewed headless runtime before any visual Canvas surface is added.

Initial candidate workflows to freeze from actual engine support:

- `character_turnaround`
- `expression_sheet`
- `pose_sheet`
- `storyboard_auto`
- `storyboard_manual`
- `hero_frame`
- `image_to_3d_bootstrap`
- `character_rig_bootstrap`
- `image_to_video_preview`
- `image_upscale`
- `video_upscale`
- `image_remove_background`
- `video_remove_background`
- `image_outpaint`
- `video_reframe`
- `video_motion_control`
- `video_clip_extract`
- `shot_render_preview`
- `game_asset_batch`
- `game_build_playtest`

Steps:

- [x] Define workflow-specific input/output schemas independently from execution-binding/provider/model catalogs for the curated PHASE-03 workflow set; unsupported parameters fail schema validation before job admission.
- [x] Define typed Creative Graph node/edge/port/revision contract around workflows/jobs/Elements.
- [x] Define dirty-descendant and partial-rerun semantics: a changed node invalidates itself plus transitive descendants while completed unaffected outputs from the prior same-graph job remain reusable.
- [x] Define a project-scoped immutable template input/output contract with `template_save`, `template_get`, and `template_list`; duplicate template identity is rejected rather than silently overwritten.
- [x] Implement the minimal headless executor needed by downstream phases: deterministic dependency resolution, bounded execution of reviewed nodes, up to 16 declared-independent nodes per parallel worker wave, node status/output/batch recording, descendant invalidation, and reuse of completed unaffected ancestors.
- [x] Map every executable node to an already reviewed capability/job/effect boundary; the graph runtime itself grants no new shell/Python/network/deploy authority.
- [x] Keep advanced template UX, selected-subgraph execution ergonomics, production Canvas UI, and broader parity hardening for PHASE-14.
- [x] Validate that graph representation cannot encode arbitrary code/endpoints/credentials as ordinary nodes.

**Validation:** 2026-09-15 Creative acceptance proves all 24 curated workflows advertise specific schemas and reject unsupported parameters, one shared headless graph runtime executes Scene visual-evidence, Anime temporal-evidence, and Game QA branches, independent DAG levels share execution batches and run through bounded scoped workers, partial rerun invalidates only changed descendants while reusing the independent branch, immutable template save/get/list round-trips declared I/O, and endpoint/credential/executable graph payloads remain rejected.

**Commit boundary:** `feat(creative): add workflow and minimal graph runtime`.

**Phase exit criteria:**

- [x] semantic capability, execution-binding, workflow, asset/history, and budget discovery are distinct and queryable;
- [x] explicit caller-selected binding works through the non-default conformance binding and omitted execution binding fails without MCP-side ranking/defaulting/auto-selection; production real-binding execution remains owned by later engine phases;
- [x] core MCP utility workflows are represented: upscale, background removal, outpaint/reframe, motion control, and clip extraction;
- [x] job lifecycle is bounded, listable/retrievable across normal turns, and owner/project scoped;
- [x] cost/compute preflight and hard budget thresholds can block a batch before execution;
- [x] graph/workflow contracts can express scene/anime/game dependencies;
- [x] the minimal headless graph executor can run shared control/evidence dependencies without Scene/Anime/Game implementing separate orchestration loops;
- [x] arbitrary upper layers can make their own routing decisions without hard-coded provider/model IDs in MCP contracts.

# PHASE-04 — Initial reference-aware media generation

**Goal:** prove the reference/identity/style generation substrate through the anime A1/A2 still benchmark before solving full 3D production, while keeping the public MCP contract independent from the conformance execution binding used for the test.

**Dependencies:** PHASE-02, PHASE-03.

### TASK-011 — Implement the media execution-binding contract and core image utility surface

**Outcome:** at least one conformance binding can perform reference-aware image generation/editing plus image-side MCP parity utilities, while the public capability contract remains independent of that binding/provider/model.

**Steps:**

- [x] Freeze one provider/model-neutral execution-binding interface for media operations before choosing any conformance implementation.
- [x] Confirm each registered binding's operator setup, endpoint policy, executable/plugin trust model, result format, cancellation, license metadata, and credentials remain implementation details. *(Public descriptors stay semantic/credential-free; operator-only backend mapping is separate.)*
- [x] Implement/route supported image capabilities through the same job/lineage contract: generate, reference-generate, edit/inpaint, upscale, remove-background, and outpaint; pose/depth controls remain capability-gated per binding.
- [x] Make execution-binding discovery expose supported utility/reference roles and validated extensions without changing semantic tool names.
- [x] Keep arbitrary executor-native graphs/custom code disabled in ordinary capability calls.
- [x] Add operator registration/configuration docs without declaring a product-wide default provider/model.

**Validation:** 2026-09-15 acceptance uses the existing debug/test conformance binding for generic lifecycle semantics and an operator-selected `binding_local_raster=local_raster` implementation for real contained image execution. All seven reviewed image capabilities produce decodable PNG candidate Assets through the same submit/wait contract with job/parent/Element lineage and bounded dimensions/output bytes; removing the private backend mapping fails closed with `execution_not_implemented`. Public tool/capability names remain unchanged and no provider/model/Higgsfield dependency is introduced. Dependency audit additionally upgraded `rustls` to 0.23.45 and `chacha20` to 0.10.2; `cargo audit` reports no findings.

**Commit boundary:** `feat(creative): add media execution binding`.

### TASK-012 — Implement structured prompt/spec compiler

**Outcome:** an upper-layer-authored, engine-neutral creative request compiles deterministically into the caller-selected binding's validated input shape with traceable versioning.

**Steps:**

- [x] Define provider/model-neutral semantic request spec.
- [x] Define reference role/order.
- [x] Accept creative prompt/content from the upper layer; do not author or enhance it inside MCP.
- [x] Translate only reviewed semantic fields plus namespaced binding extensions.
- [x] Record binding/compiler version in job lineage.
- [x] Support controlled changed-fields for revision requests.

**Validation:** 2026-09-15 acceptance proves `creative.semantic.v1` rejects unknown semantic fields, duplicate/invalid reference order, unsupported changed fields, invalid namespaced extensions, and raw-parameter/semantic-spec mixing; the same provider-neutral request compiles deterministically through two explicitly selected mock bindings with different binding versions/extensions, preserves typed reference role/order, and persists compiler version, binding version, and sorted changed-fields in durable job lineage across later retrieval. MCP copies caller-authored prompt/content verbatim and does not add creative prompt strategy or provider/model names.

**Commit boundary:** `feat(creative): compile structured generation specs`.

### TASK-013 — Prove Character/Style Element execution primitives with an upper-layer-driven still workflow

**Outcome:** an external client/agent can use MCP state/jobs/assets to create and curate a stable front/side/back/three-quarter/expression reference set without MCP owning interviews, style decisions, prompt strategy, or model choice.

**Steps:**

- [x] Accept a caller-authored Style/Character specification and selected execution binding.
- [x] Store the caller-approved Style Element revision.
- [x] Execute requested canonical/reference views and additional views/expressions with typed reference roles.
- [x] Label generated hidden views as interpreted.
- [x] Return media plus deterministic metadata/evidence for upper-layer visual review.
- [x] Let the caller promote accepted revisions into the Character Element/Pack through explicit state mutation.

**Validation:** 2026-09-15 deterministic upper-layer acceptance drives Style and Character Element revisions plus `character_turnaround` and `expression_sheet` through the same agnostic job/media contracts, producing contained candidate Assets with Anime surface, Element/reference lineage, interpreted authority, per-variant dimensions/checksum evidence, and explicit `inspection=not_inspected`. The caller can explicitly reject Assets/revisions or promote accepted Assets/revisions; MCP never chooses creative direction/model or claims subjective QA. This proves the TASK-013 contract and lifecycle, **not** production character-consistency quality: PHASE-04 A1/A2 visual-quality exit criteria remain open until evaluated with a real quality-capable caller-selected binding/evaluator.

**Commit boundary:** `feat(anime): build character reference workflow`.

**Phase exit criteria:**

- [x] one character can be reproduced across controlled stills;
- [x] style and character state are reusable;
- [x] visual QA can reject/promote revisions;
- [x] accepted turnaround is ready for 3D production.

# PHASE-05 — Identity, Style, World, and Element production system

**Goal:** turn the first still workflow into reusable Character/Location/Prop/Style Elements rather than demo prompts, so the same state can feed Scene Studio, Anime Studio, and Game Studio.

**Dependencies:** PHASE-04.

### TASK-014 — Add dependency-aware Style Pack revisions

**Outcome:** style changes identify affected downstream assets rather than silently drifting the project.

**Implementation:** COMPLETE on 2026-09-15. Assets can carry bounded validated `dependency_element_ids`; Character/Anime reference workflows declare the selected Style Element separately from their primary Character Element binding. Promoting a different accepted Style revision creates bounded `style_dependency` soft-review findings for only declared dependent Assets and Scene shots using that Style Element, without automatically changing Asset acceptance state or unrelated project state. Re-promoting the same Style revision is idempotent and does not duplicate findings; review-capacity overflow fails closed.

**Validation:** deterministic acceptance changes palette rules on a persisted Style Element after creating style-dependent character reference Assets and a valid Scene/shot fixture. Dependent candidate Assets and the affected shot receive soft review findings, an unrelated accepted Asset remains accepted and unflagged, and repeated promotion of the same revision leaves finding count unchanged. Creative suite: 23/23 PASS; fast guardrail PASS.

**Commit boundary:** `feat(creative): track style dependencies`.

### TASK-015 — Support optional identity-capable execution bindings without replacing Character Pack authority

**Outcome:** embeddings/LoRA/fine-tunes or future identity mechanisms can improve consistency while remaining implementation details of caller-selected bindings.

**Steps:**

- [x] Measure reference-only baseline first.
- [x] Expose training/identity preparation only when an upper layer explicitly requests a compatible binding/capability; MCP never decides that training is needed.
- [x] Store training artifact/license/version lineage.
- [x] Keep Character Pack references and design constraints authoritative.

**Validation:** 2026-09-15 acceptance proves the provider-neutral `identity.prepare` capability is absent as an execution route unless a caller explicitly selects a compatible binding, and real-person preparation fails schema validation without `authorization_attested=true`. A debug/test-only identity binding materializes a contained candidate `identity_binding_artifact` with Character Element, parent-reference, job, selected binding/version, artifact kind/version, and binding license lineage. Rejecting that artifact and replacing the binding/artifact version leaves the Character Element's selected revision, reference Asset IDs, and design spec unchanged. Raw bound jobs now persist execution-binding version lineage even when no semantic compiler is involved. Creative suite: 25/25 PASS; fast guardrail PASS.

**Commit boundary:** `feat(anime): support identity-capable execution bindings`.

### TASK-016 — Add World/Location Pack

**Outcome:** recurring locations/environment language can be reused across key art and shots.

**Implementation:** COMPLETE on 2026-09-15. Location Element revisions now use the strict `world_location_v1` contract with bounded concept/scale, architecture and set-dressing language, canonical landmarks, time/weather/lighting variants, optional Style and Asset3d Element dependencies, reusable camera landmarks, and continuity notes. `ShotManifest.location_variant_id` is a typed optional reference to one declared variant from the Scene's selected Location Pack; unknown variants, wrong Element kinds, unknown Style/Asset3d dependencies, duplicate identities, and ambiguous extra spec fields fail closed.

**Validation:** deterministic acceptance creates/promotes/reloads a typed Location Pack through `creative_element`, then runs two durable key-art generation jobs bound to the same Location Element and Style dependency without changing the selected Location revision. The cross-track Scene fixture references that same Location Element from multiple shots while selecting `day_clear` versus `rain_night` and distinct camera settings; an undeclared shot variant is rejected. Creative suite: 26/26 PASS; fast guardrail PASS.

**Commit boundary:** `feat(anime): add reusable world packs`.

**Phase exit criteria:**

- [x] style/character/world state survives multiple jobs;
- [x] binding-specific identity preparation is optional, traceable, and replaceable;
- [x] downstream impact of style changes is explicit.

# PHASE-06 — Blender first-class production engine

**Goal:** implement the retained original Plan 069 Blender capability on the current single runtime capability/catalog framework.

**Dependencies:** PHASE-02, PHASE-03.

### TASK-017 — Freeze Blender bridge/config/session/tool schemas

**Outcome:** operator config, reviewed executable resolution, explicit bounded `blender_session status|start|stop`, loopback framing, five public bounded Blender MCP tool schemas plus foreground operator-only heavy operations, dedicated project `blender/` layout, inspection scopes, docs source, bounds, effects, and approval policy are frozen against current Blender Lab integration.

**Steps:**

- [x] Freeze operator-configured Blender executable plus bounded reviewed fallback discovery; no caller-supplied arbitrary executable path.
- [x] Freeze owner-safe session identity so an existing user-started Blender can be attached to but never killed by relay `stop`.
- [x] Freeze readiness semantics: probe compatible loopback bridge first; explicit `start` launches Blender only when needed and waits for bounded bridge readiness.
- [x] Freeze the canonical `<project>/blender/` production layout and require every Blender-owned save/render/export/checkpoint/bake/temp destination to resolve beneath it.
- [x] Keep executable authority separate from project-data authority: Blender may execute from an approved system/home installation path while production artifacts stay under the authorized Projects workspace.

**Validation:** UPDATED on 2026-09-16. Blender exposure is now governed solely by the master `RELAY_ENABLE_CREATIVE` flag; there is no separate Blender enable flag. Operator-only executable, bridge port, and bridge timeout configuration remain bounded and caller schemas expose no host/port/executable/PID injection. The frozen Blender Lab protocol identity is loopback TCP on operator-selected port (default 9876) with NUL-delimited bounded JSON framing. Session state distinguishes external attachment from relay-owned launch so stop semantics can never assume ownership of a user-started Blender. The canonical `blender/{scenes,assets,references,renders/preview,renders/final,animations,exports,checkpoints,tmp}` subtree and artifact-scope helpers reject absolute paths, traversal, and noncanonical destinations. The public Blender MCP catalog is now intentionally limited to five bounded tools (`blender_session`, `blender_inspect`, `blender_python_api_docs`, `blender_screenshot`, `blender_animation_preview`). The preview surface is a bounded sampled temporal-review path; heavy/bulk preview execution remains available through the operator CLI. Arbitrary Python, final render, import/export, and checkpoint operations remain first-party operator-CLI-only capabilities and are not exposed through `tools/list`. Blender contract acceptance: 5/5 PASS; CLI config acceptance PASS; focused Blender contract/read tests, Clippy `-D warnings`, fast Rust/auto guardrails, and `git diff --check` have passed during the current implementation sequence.

**Commit boundary:** `feat(blender): freeze relay capability contract`.

### TASK-018 — Implement bounded Blender session lifecycle and loopback bridge

**Outcome:** Rust relay can attach to an already-running compatible Blender or explicitly launch the reviewed Blender executable when closed, then communicate only through the reviewed local add-on protocol with bounded framing/errors.

**Implementation:** COMPLETE on 2026-09-15. The relay now speaks the reviewed Blender Lab one-request-per-connection loopback TCP contract directly (`execute` JSON framed by NUL bytes), validates a fixed relay-authored readiness payload, bounds connect/write/read timing and frame sizes, and never accepts caller host/port/executable/PID authority. Session state distinguishes external versus relay-owned bridges. Cold start resolves only an operator-configured absolute executable or the reviewed safe-PATH Blender fallback, initializes the canonical project `blender/` layout idempotently, constrains workflow temp environment to `blender/tmp/`, launches the process in a dedicated process group where supported, and polls bounded readiness. Relay-owned identity is tied to authenticated owner + Creative project ID + workspace root; stop refuses external or mismatched sessions and process-group cleanup kills relay-owned descendants. Losing relay state therefore cannot convert an external/user process into kill authority.

**Steps:**

- [x] Implement bounded executable resolution from operator config/reviewed standard locations only.
- [x] Implement `blender_session status` as a non-mutating bridge/process readiness probe.
- [x] Implement explicit `blender_session start` that launches the reviewed executable, points controlled session temp/project context at `<project>/blender/`, and waits for compatible loopback bridge readiness.
- [x] Track relay-owned process identity strongly enough that `blender_session stop` can terminate only a process launched by this session manager.
- [x] If a compatible external/user-started Blender is already available, attach without taking ownership and refuse destructive stop.
- [x] Keep all bridge targets loopback-only and reject caller-provided host/port/executable overrides outside frozen operator config.

**Validation:** deterministic fake process/bridge acceptance covers external attach, cold start to ready, relay-owned status/stop, wrong-owner and wrong-project refusal, descendant process-group cleanup, idempotent layout creation, valid NUL-delimited framing, incompatible/malformed/oversized/hung responses, startup failure, readiness timeout, and schema-level impossibility of caller host/port/executable/PID injection. Blender-focused platform tests: 5/5 PASS; Clippy, maintainability, and `git diff --check` PASS. Explicit stop provides the supported cancellation boundary for relay-owned sessions; there is no separate caller-supplied process cancellation primitive.

**Commit boundary:** `feat(blender): add loopback bridge`.

### TASK-019 — Implement structured Blender read/knowledge/preview tools

**Outcome:** session status, inspection scopes, version-aligned API/manual lookup, screenshots, animation previews, and bounded render previews work without arbitrary Python input; previews materialized by the workflow remain under `<project>/blender/renders/preview/` or another frozen Blender-subtree path.

**Implementation:** COMPLETE on 2026-09-15. `blender_inspect` covers all frozen scene/object/mesh/uv/rig/animation/material/nodes/physics/asset/render/character scopes through relay-authored bounded `bpy` programs; `blender_python_api_docs` performs bounded live `bpy` namespace introspection and reports the matching Blender-version API documentation base without granting network authority. Screenshot and sampled animation-preview tools write only project-contained PNGs under `blender/renders/preview/`, enforce frame/dimension/total-pixel/file-size bounds, reject existing/symlink/escaped targets, decode the resulting image, and return relative-path/dimension/byte/SHA-256 evidence.

**Validation:** deterministic fake-bridge acceptance exercises every frozen inspection scope plus version-aligned docs, contained screenshot, sampled temporal preview, checksums, invalid docs namespace rejection, and overwrite refusal. Blender-focused platform tests: 6/6 PASS; fast guardrail and maintainability PASS.

**Commit boundary:** `feat(blender): add structured production reads`.

### TASK-020 — Implement contained Blender asset/render/recovery paths

**Outcome:** `.blend` saves, materialized references, import/export, render outputs, controlled bake/cache outputs, and checkpoint create/restore respect workspace/protected-path authority and the canonical `<project>/blender/` layout.

**Implementation:** COMPLETE on 2026-09-15. Structured Blender import first materializes registered Creative Assets into `blender/references/` or `blender/assets/` through the existing contained binary-write primitive and registers explicit `blender_materialized` lineage before relay-authored import code runs. Still/fixed-frame animation renders are limited to canonical preview/final paths, reusable animation exports are separated from ordinary exports, and all generated files are post-verified as bounded regular files beneath the canonical Blender subtree before candidate Blender-surface Assets are registered. Checkpoint creation writes a scene snapshot beneath `blender/scenes/`, copies recovery state beneath `blender/checkpoints/`, and stores owner/project/checksum receipt state under protected Creative state rather than mutable production directories; restore verifies protected receipt, Asset lineage, owner/project identity, path, and checksum before opening the file.

**Steps:**

- [x] Materialize project-contained external references into `<project>/blender/references/` or `<project>/blender/assets/` before Blender consumes them.
- [x] Require scene saves under `<project>/blender/scenes/`.
- [x] Require preview/final renders under `<project>/blender/renders/preview|final/`.
- [x] Require reusable animation outputs under `<project>/blender/animations/` and exports under `<project>/blender/exports/`.
- [x] Require checkpoints under `<project>/blender/checkpoints/` and controlled workflow temp/bake paths under `<project>/blender/tmp/` unless a more specific frozen subdirectory is defined.
- [x] Reject absolute/arbitrary host destinations even when Blender itself would accept them.

**Validation:** deterministic fake-bridge acceptance covers materialization/import, preview/final renders, export versus reusable animation output, scene snapshot + checkpoint creation, correct-owner restore, wrong-owner rejection, checksum tamper rejection, path traversal, unsupported format, and symlink output refusal. Blender platform suite: 7/7 PASS; Clippy `-D warnings`, fast guardrail, maintainability, and `git diff --check` PASS.

**Commit boundary:** `feat(blender): add contained asset and checkpoint tools`.

### TASK-021 — Implement privileged Blender Python authoring

**Outcome:** `blender_execute_python` enables coherent live modeling/rigging/animation edits with high-risk/manual semantics.

**Implementation:** COMPLETE on 2026-09-15. Caller-authored Blender Python is accepted only through the dedicated high-risk tool, capped by the frozen 256 KiB code bound, rejects NUL framing injection, and executes through the same reviewed loopback bridge rather than through shell/stdio or an alternate Python server. Result mode is explicit (`json|text`), encoded results are capped at 1 MiB, bridge stdout remains subject to the bridge response bound, and every result explicitly reports `sandboxed=false` plus `authority=blender_host_user` so Blender host-user execution is never misrepresented as contained execution. Tool annotations remain mutating/open-world; effect policy records process execution, workspace read/write, external mutation, and privileged bridge authority. Activity presentation intentionally excludes raw `code`, so caller source/secrets are not copied into the activity journal.

**Validation:** deterministic fake-bridge acceptance executes JSON and text authoring calls, verifies strict-json framing, bounded results, explicit unsandboxed authority metadata, high-risk effect classes/tool annotations, and activity redaction; oversized code fails before bridge use. The composed Blender platform suite passes 8/8, all-target/all-feature Clippy passes with `-D warnings`, and fast Rust/auto guardrails plus `git diff --check` pass. Owner-aware dispatch fallout in three historical Rust acceptance examples was reconciled by supplying their local fixture owner rather than adding a compatibility API.

**Commit boundary:** `feat(blender): add privileged authoring bridge`.

### TASK-022 — Compose optional capability/resources/docs

**Outcome:** Blender appears only when enabled; capability resources, activation hints, routing guidance, docs, and tool catalog agree.

**Implementation:** UPDATED on 2026-09-19. The single canonical `runtime_tool_catalog` composes five bounded public Blender MCP tools for the Full profile whenever the single Creative master flag is enabled; Primary and disabled configurations remain unchanged. The fifth tool is `blender_animation_preview`, a bounded sampled temporal-preview surface with contained PNG outputs and bounded inline image delivery. Arbitrary Python, final render, import/export, and checkpoint execution remains available only through the synchronous foreground `ai-tools creative` operator CLI. A read-only `blender-capability` resource is configuration-gated and documents the bounded-MCP versus foreground-operator boundary without exposing operator executable, bridge port, credentials, or other hidden authority.

**Validation:** refreshed catalog acceptance proves no Blender tool leakage while disabled, exactly five bounded Blender MCP tools when enabled, six heavy Blender operations absent from public `tools/list`, no Blender exposure in Primary, and the optional resource only when the capability is active. Focused Blender contract/read acceptance passes with the five-tool catalog including `blender_animation_preview`; the final 2026-09-20 full repository gate also passes after the completed source/doc changes.

**Commit boundary:** `feat(blender): compose optional capability`.

**Phase exit criteria:**

- [x] all required Blender capabilities are implemented/tested, with five bounded operations public through MCP (including sampled `blender_animation_preview`) and heavy execution available through the synchronous foreground operator CLI;
- [x] no generic stdio/nested Python MCP server is required;
- [x] Blender filesystem/network/process/host authority is represented honestly;
- [x] the Blender executable may remain outside Projects only as reviewed operator executable authority, while every workflow-owned production artifact remains beneath the selected project's dedicated `blender/` subtree;
- [x] safe reference materialization integrates with Blender import.

# PHASE-07 — Anime 3D character production

**Goal:** reach A3/A4 using the accepted Character/Style Elements/Packs and Blender engine.

**Dependencies:** PHASE-05, PHASE-06.

### TASK-023 — Implement 2D-reference -> Blender character bootstrap workflow

**Outcome:** accepted turnaround references become a contained Blender production setup.

**Steps:**

- [x] Materialize/import front/side/back references.
- [x] Accept the bootstrap route chosen by the upper layer: caller-selected image-to-3D execution binding, Blender/manual blockout, or hybrid.
- [x] Validate that the selected route/binding is active and compatible; MCP does not rank bootstrap methods or choose one from quality heuristics.
- [x] Preserve source/reference alignment metadata.
- [x] Inspect multi-angle silhouette before detail work.

**Validation:** fixture can reproduce the same bootstrap from project state and contained assets.

**Commit boundary:** `feat(anime): bootstrap character in blender`.

### TASK-024 — Production mesh/UV/material pass

**Outcome:** character becomes a reusable deformation-ready asset rather than a generated mesh artifact.

**Steps:**

- [x] Inspect topology/normals/transforms.
- [x] Retopologize or repair as required.
- [x] Establish UV readiness.
- [x] Build toon/stylized material system from Style Pack.
- [x] Add eyes/hair/clothing/accessories with chosen strategy.
- [x] Capture multi-angle visual QA.
- [x] Checkpoint accepted asset state.

**Validation:** mesh/UV/material/character inspection + rendered views meet fixture gates.

**Commit boundary:** `feat(anime): produce reusable character asset`.

### TASK-025 — Rig, weights, facial controls

**Outcome:** character supports representative body and facial animation.

**Steps:**

- [x] Build/reuse reviewed humanoid armature strategy.
- [x] Skin and inspect weights.
- [x] Add constraints/IK/FK only where workflow benefits.
- [x] Test representative deformation poses.
- [x] Add eye/jaw/expression controls.
- [x] Checkpoint and export contained reusable asset.

**Validation:** A4 deformation/facial fixture passes structural + visual review.

**Commit boundary:** `feat(anime): rig reusable character`.

**Phase exit criteria:**

- [x] one project character is reusable in a fresh scene;
- [x] topology/UV/material/rig/facial state is inspectable;
- [x] representative deformation is visually accepted;
- [x] asset can be exported and re-imported within contained workspace authority.

# PHASE-08 — Animation, facial, and temporal QA

**Goal:** reach A5 with real motion, not only rig existence.

**Dependencies:** PHASE-07.

### TASK-027 — Add pose/action execution primitives

**Outcome:** any upper layer can author a short action using inspected rig state, API docs/resources, and durable project state; MCP exposes bounded inspection/control state while Blender authoring/preview that can exceed 60 seconds is synchronous foreground operator work.

**Validation:** action/F-curve/keyframe/NLA state is structurally visible and representative frames show intended motion.

**Commit boundary:** `feat(anime): add character action workflow`.

### TASK-029 — Add secondary motion review

**Outcome:** hair/clothing secondary motion can be authored or deliberately omitted with explicit reasoning.

**Validation:** physics/constraint state is inspectable and clipping/explosion failures are caught in sampled preview.

**Commit boundary:** `feat(anime): add secondary motion workflow`.

**Phase exit criteria:**

- [x] a 3–5 second character performance exists;
- [x] structural animation state and temporal preview agree;
- [x] facial/body motion remain separately editable;
- [x] failures can roll back to a checkpoint.

# PHASE-09 — Scene Studio state/execution: SceneBoard, Director specification, and shots

**Goal:** reach S1/S2 and produce an S3 candidate while keeping storyboard/directing intelligence above MCP: MCP stores/validates caller-authored SceneBoard/Director state and admits bounded job state; requested shot generation/rendering that can exceed 60 seconds runs synchronously in the foreground operator CLI.

**Dependencies:** PHASE-04, PHASE-05. PHASE-06/08 are required only for Blender-backed animated shots; generated-video scenes use caller-selected compatible execution bindings.

### TASK-030 — Implement SceneBoard + Scene/Shot Manifest contracts

**Outcome:** MCP can store, validate, version, and revise a connected board and bounded scene/shot plan supplied by any upper layer before render-heavy work starts.

**Steps:**

- [x] Support `planning_mode=auto|manual` as provenance metadata, but **do not implement AI auto-planning inside MCP**: in `auto`, the upper layer supplies the generated board; in `manual`, the upper layer/user supplies explicit per-frame/shot direction.
- [x] Validate the same board/shot schema regardless of which upper layer produced it.
- [x] Bind Character/Location/Prop/Style Elements with selected revisions.
- [x] Track global scene look/lighting/atmosphere/spatial constraints across frames.
- [x] Allow frame-level revision while preserving unrelated accepted frames.
- [x] Promote selected frames to hero-frame candidates.
- [x] Freeze shot IDs/durations/assets/continuity/camera intent.
- [x] Validate required upstream Elements/assets exist.
- [x] Keep approval/effect boundaries explicit; conversational autonomy policy remains upper-layer behavior.

**Validation:** two different upper-layer fixtures (one labeled auto, one manual) can submit/revise the same SceneBoard contract without hidden chat context; changing one frame preserves other accepted frame identities/lineage.

**Commit boundary:** `feat(scene): add sceneboard and shot planning`.

### TASK-031 — Implement Director-spec validation + generated-video utility surface + shot execution DAG

**Outcome:** caller-authored global scene settings and per-shot cinematography can be validated/executed through selected bindings, while generated-video executors expose reusable transform/job semantics expected from the MCP parity matrix.

**Steps:**

- [x] Add global scene settings: genre/look, Style Element, lighting, color palette, atmosphere, era/time where relevant.
- [x] Add per-shot settings: shot size/framing, camera profile, lens/focal/aperture/DoF intent, move, movement speed/stabilization, tempo/edit intent.
- [x] Store a caller-selected Hero Frame First route when requested; MCP does not decide when it is creatively beneficial.
- [x] Prepare/admit a shot through the caller-selected generated-video binding, Blender path, or explicit mixed graph without changing the engine-neutral manifest; execute heavy shot work through the exact synchronous foreground operator command.
- [x] Route active video capabilities through semantic capability/execution-binding/job/asset contracts: text/reference/image-to-video generation, extend, reframe, upscale, remove-background, and motion-control when supported.
- [x] Validate distinct media roles for motion-control inputs (character/reference image vs motion-reference video) and preserve timestamp/duration lineage.
- [x] Expose video utility workflows outside Scene Studio too; a user should be able to reframe/upscale/remove-background an existing Asset without manufacturing a Scene Manifest.
- [x] MCP accepts already-authored Director settings/prompts from the upper layer; it neither invents Director suggestions nor silently triggers generation.
- [x] One failed shot/utility transform invalidates/retries only its affected descendants.

**Validation:** the same Scene Manifest can execute through two caller-selected mock video bindings and a fake/real Blender path with consistent Element/camera intent; separate fixtures prove reframe, upscale, background removal, and motion-control child-asset lineage/job semantics without MCP choosing the backend.

**Commit boundary:** `feat(scene): add director and shot orchestration`.

### TASK-032 — Implement scene continuity and carry-forward anchors

**Outcome:** cross-shot character, costume, prop, location, screen direction, lighting, action, camera, and state continuity is reviewed before final output.

**Steps:**

- [x] Compare Element revisions and scene-enter/exit state across shots.
- [x] Track location geography/light direction/important landmarks.
- [x] Track action/prop handoff and screen direction.
- [x] Track selected hero/reference anchor used to carry continuity into later generated sequences.
- [x] Emit bounded findings that can be resolved with scoped revision.

**Validation:** seeded scene continuity fixtures produce detectable findings, and a selected prior frame can be registered as the next-sequence anchor without overwriting original provenance.

**Commit boundary:** `feat(scene): validate shot continuity`.

**Phase exit criteria:**

- [x] S1 SceneBoard state supports upper-layer Auto/Manual experiences through one agnostic contract;
- [x] S2 global/per-shot Director controls are represented independently from engines and authored outside MCP;
- [x] one 10–30 second S3 candidate has bounded shot state;
- [x] reusable Character/Location/Prop/Style Elements survive shot changes;
- [x] generated-video and/or Blender preview can be inspected and revised shot by shot;
- [x] active video utilities (reframe/upscale/remove-background/motion-control where supported) are independently callable through normal capability/execution-binding/workflow/job/asset contracts.

# PHASE-10 — Unified QA evidence and surgical revisions

**Goal:** make structural/deterministic evidence and lineage-preserving revision a shared MCP contract while subjective visual/creative judgment remains upper-layer responsibility unless a caller explicitly selects an evaluator binding.

**Dependencies:** PHASE-04, PHASE-08.

### TASK-033 — Implement QA finding schema

**Outcome:** still, 3D, render, and temporal review emit bounded domain findings with severity and source revision.

**Validation:** findings distinguish hard fail, soft finding, and not-inspected.

**Commit boundary:** `feat(creative): add production qa findings`.

### TASK-034 — Implement visual QA evidence contract

**Outcome:** MCP returns the images/previews/reference state and deterministic checks needed for an upper layer to judge Character/Style/Shot invariants; optional machine evaluation is only run through a caller-selected evaluator binding.

**Validation:** without an evaluator binding, subjective fields are `not_inspected` rather than fabricated; with a mock caller-selected evaluator, findings attach to the correct Asset/revision without changing MCP semantics.

**Commit boundary:** `feat(creative): add visual qa workflow`.

### TASK-035 — Implement temporal QA evidence contract

**Outcome:** MCP produces bounded frame samples/playblast/timing/structural metadata for upper-layer review; optional semantic evaluation requires an explicit caller-selected evaluator binding.

**Validation:** deterministic timing/contact metadata is returned directly; semantic identity/motion judgments are `not_inspected` without an evaluator and attach correctly when a mock binding is explicitly selected.

**Commit boundary:** `feat(creative): add temporal qa workflow`.

### TASK-036 — Implement surgical revision lineage

**Outcome:** a caller can request one scoped child revision while preserving other locked production state.

Example scopes:

- expression only;
- pose adjustment;
- background/location lighting;
- camera framing;
- palette accent;
- hair/clothing clipping fix;
- one shot's timing;
- one material/shader correction.

**Validation:** child revision declares changed scope and parent; unrelated locked fields remain unchanged or failures are reported.

**Commit boundary:** `feat(creative): add scoped revision workflow`.

**Phase exit criteria:**

- [x] generation success is no longer conflated with QA success;
- [x] revision lineage is first-class;
- [x] accepted work can be refined without full regeneration by default.

# PHASE-11 — Scene/anime composite, export, and reusable delivery

**Goal:** turn accepted Scene Studio / Anime Studio shots into contained deliverables and reusable Elements/assets without hidden deployment/publication.

**Dependencies:** PHASE-09, PHASE-10.

### TASK-037 — Implement deterministic sequence assembly and Personal-Clipper-style extraction

**Outcome:** accepted shot renders can be assembled deterministically, and an existing long-form video Asset can be analyzed/segmented into reusable clips without bypassing normal ingest, job, and lineage rules. Assembly/extraction execution is synchronous foreground operator work whenever accepted input size/duration can exceed the 60-second agent ceiling; MCP owns manifests, admission, and result lineage.

**Steps:**

- [x] Assemble accepted shots with explicit ordering, frame rate, resolution, and transitions.
- [x] Implement `video.clip_extract` over a contained/imported source Asset with bounded duration/input size.
- [x] Preserve source start/end timestamps, transcript/segment metadata where available, parent Asset ID, and selected clip revisions.
- [x] Allow explicit user-selected time ranges and an optional reviewed clip-selection workflow; do not silently download arbitrary third-party media outside the safe URL-import path.
- [x] Register extracted clips as normal child Assets that can feed SceneBoard, Canvas, Game, or export workflows.

**Validation:** deterministic sequence fixture assembles identically from manifest state; clip-extraction fixture produces timestamp-addressable child Assets and rejects unsupported/oversized/unowned source media.

**Commit boundary:** `feat(anime): assemble accepted sequence`.

### TASK-038 — Implement final export profiles

**Outcome:** reviewed profiles produce contained video/still/project outputs and reusable character/world assets.

**Validation:** output paths cannot escape workspace; profile metadata is recorded; no upload occurs implicitly.

**Commit boundary:** `feat(creative): add contained export profiles`.

### TASK-039 — Implement project handoff bundle

**Outcome:** final delivery includes enough human-readable state to continue production later: manifests, selected assets, Blender project/checkpoint/export, final media, and bounded QA summary.

**Validation:** a fresh agent session can inspect the handoff and identify current accepted state without relying on previous conversation memory.

**Commit boundary:** `feat(creative): add production handoff bundle`.

**Phase exit criteria:**

- [x] final media and reusable project assets are contained;
- [x] long-form source video can produce reusable timestamp-lineaged clip Assets through the bounded clip-extraction workflow;
- [x] no publish/deploy action is hidden in export;
- [x] a later session can continue from project state.

# PHASE-12 — Game Studio: design, assets, build, and playtest

**Goal:** reach G1/G2/G3 with one small but complete browser game using the same Elements, Style, Asset, job, and QA foundations as Scene/Anime Studio.

**Dependencies:** PHASE-02, PHASE-03, PHASE-05. PHASE-06 is optional for games that need Blender-authored 3D/rigging.

### TASK-040 — Implement Game Design/Build Manifest validation and state

**Outcome:** MCP can validate/store/version an upper-layer-authored Game Design/Build Manifest and STYLE FORMULA before broad asset execution or build work.

**Steps:**

- [x] Accept the design-only/assets-only/build/deploy intent resolved by the upper layer.
- [x] Validate required genre, perspective, target devices, core loop, verbs, win/lose/restart/progression, player count, controls, camera, language, physics/timing, and performance/asset budgets.
- [x] Freeze solo/local/online multiplayer route.
- [x] Bind shared Character/Location/Prop/Style Elements where applicable.
- [x] Write stable `design/assets` roles/paths before generated asset batches.
- [x] Define placeholder policy and missing-asset behavior.
- [x] Keep public publish outside the build/deploy workflow.

**Validation:** two different upper-layer clients can submit the same valid G1 manifest and drive identical MCP build-state behavior; required gameplay/style/input fields fail deterministically when absent, without MCP asking conversational questions.

**Commit boundary:** `feat(game): add game production manifest`.

### TASK-041 — Implement parallel game asset generation and source integration

**Outcome:** image/3D generation jobs can run independently while editable game source is built against stable manifest identities.

**Steps:**

- [x] Add game-specific media roles for sprites, UI, tileables, sky/environment, textures, 3D models, and animation clips.
- [x] Execute independent asset jobs concurrently only when the caller-specified graph/manifest declares them independent; MCP does not creatively schedule undeclared work.
- [x] Build source against manifest-stable placeholders/paths rather than ephemeral job filenames.
- [x] Promote accepted results into Elements/assets and replace placeholders deterministically.
- [x] Validate skeleton/action compatibility for animated 3D assets; route to Blender/retargeting when necessary.
- [x] Preserve source and asset lineage separately.

**Validation:** a fixture can replace delayed generated assets after code exists without manual filename edits or source-wide rebuilds unrelated to those assets.

**Commit boundary:** `feat(game): integrate generated production assets`.

### TASK-042 — Implement reviewed browser-game source/runtime templates

**Outcome:** any upper-layer coding system can create/edit a normal workspace project for a small 2D or 3D browser game using MCP workspace/build/runtime primitives without depending on an opaque hosted builder or MCP-managed coding agent.

**Steps:**

- [x] Audit current web/coding stack before freezing templates/runtime family.
- [x] Keep simple 2D and 3D paths separate when their runtime needs materially differ.
- [x] Keep game source human-editable and versionable.
- [x] Bind asset/runtime paths through the manifest.
- [x] Add reviewed resize/input/timing patterns rather than unconstrained generated infrastructure.
- [x] Avoid hard-coding one framework into the Creative Project contract.

**Validation:** one 2D-or-3D fixture builds and serves locally through normal repository/runtime tooling with no Higgsfield dependency.

**Commit boundary:** `feat(game): add browser game runtime templates`.

### TASK-043 — Implement Game QA / local playtest contract

**Outcome:** a build cannot be called complete from compile success alone.

**Steps:**

- [x] Serve the built game over HTTP/runtime preview.
- [x] Verify start -> core loop -> win/lose -> restart.
- [x] Detect missing assets and uncaught console/runtime errors.
- [x] Verify responsive/canvas render and declared keyboard/mouse/touch/gamepad inputs.
- [x] Verify fixed-step/timing/physics assumptions where relevant.
- [x] Measure bounded performance indicators chosen during implementation.
- [x] Emit hard-fail/soft/not-inspected Game QA findings with source/build revision.

**Validation:** seeded broken-loop, missing-asset, input, console-error, and timing fixtures fail correctly; G3 benchmark requires a passing complete loop.

**Commit boundary:** `feat(game): add browser playtest qa`.

**Phase exit criteria:**

- [x] G1 game design/style/assets are frozen before broad work;
- [x] G2 source and generated assets integrate through stable identities;
- [x] G3 small browser game passes complete-loop local playtest;
- [x] source remains editable and resumable by a fresh agent session.

# PHASE-13 — Game multiplayer, deploy, and source-preserving iteration

**Goal:** add the high-value Supercomputer/Games behavior after the single-player/local core is proven: reviewed multiplayer when selected, shareable deploy, and continued source iteration without conflating deployment with publication.

**Dependencies:** PHASE-12.

### TASK-044 — Implement multiplayer route and room/state-sync module

**Outcome:** Game Studio can distinguish solo, same-screen/local multiplayer, and online multiplayer without giving generated game code unrestricted infrastructure authority.

**Steps:**

- [x] Keep local multiplayer inside the normal client/game runtime when possible.
- [x] Define one platform-owned bounded online room/state-sync interface when online mode is enabled.
- [x] Define room identity, join/leave, authoritative/shared state, update bounds, reconnect/failure behavior, and cleanup.
- [x] Prevent generated client code from receiving deployment/database/provider credentials directly.
- [x] Add two-session acceptance fixtures for online claims.

**Validation:** two independent sessions can join one disposable room and observe the reviewed synchronized state; cross-room/owner leakage fails closed.

**Commit boundary:** `feat(game): add bounded multiplayer runtime`.

### TASK-045 — Implement game deploy as a distinct lifecycle

**Outcome:** a QA-passing build can become a shareable playable deployment while public listing/publish remains a separate action.

**Steps:**

- [x] Reuse current application/deployment ownership where practical instead of inventing a second hosting plane.
- [x] Freeze deploy input to one accepted build revision.
- [x] Return deployment identity/URL and bounded status.
- [x] Preserve source/project link to the deployed build.
- [x] Never publish to a public gallery/marketplace implicitly.
- [x] Treat production deployment authority according to existing product approval/policy.

**Validation:** deploy fixture targets only a reviewed deployment surface; `game.publish`/public listing is absent or requires a distinct explicit action.

**Commit boundary:** `feat(game): separate deploy from publication`.

### TASK-046 — Implement source-preserving game iteration

**Outcome:** caller-requested follow-up changes can amend an existing game rather than silently regenerating it from scratch.

**Steps:**

- [x] Inspect existing design/source/assets before mutation.
- [x] Amend the Game Manifest when requested behavior changes.
- [x] Reuse unchanged Elements/assets/source modules.
- [x] Invalidate only affected graph/jobs/build outputs.
- [x] Re-run complete playtest after gameplay-impacting changes.
- [x] Keep deployment revision history explicit.

**Validation:** a follow-up feature request changes one mechanic/asset while unrelated accepted game behavior and assets remain intact.

**Commit boundary:** `feat(game): support iterative game production`.

**Phase exit criteria:**

- [x] G4 passes whenever multiplayer is selected for the benchmark;
- [x] G5 shareable deployment is tied to one accepted source/build revision;
- [x] deploy and publish remain different authority boundaries;
- [x] follow-up iteration preserves source/project continuity.

# PHASE-14 — Mature Creative Graph runtime, Canvas workspace, and MCP parity evals

**Goal:** complete C3 and high-value Higgsfield MCP parity by hardening and expanding the minimal PHASE-03 headless graph runtime into the full reusable production executor/workspace over proven scene/anime/game primitives, without adding an agent/skill/model router.

**Dependencies:** PHASE-03 plus working scene/game workflows from PHASE-09/12. Blender/anime graph nodes depend on their owning phases.

### TASK-047 — Mature the typed Creative Graph executor

**Outcome:** the minimal PHASE-03 executor is hardened for production-scale branch/parallel/partial-rerun usage while preserving underlying tool authority and without duplicating Scene/Game orchestration.

**Steps:**

- [x] Preserve the PHASE-03 dependency resolution, bounded concurrency, invalidation, and authority model as the single graph execution path.
- [x] Harden deterministic foreground scheduling/recovery for larger graphs and mixed long-running jobs within existing admission bounds; do not introduce an async/background agent executor.
- [x] Enrich node run state with output Asset IDs, QA, cost/compute metadata, and failure classification where available.
- [x] Support selected-node/subgraph/all-dirty execution through synchronous foreground operator commands while preserving durable state for later MCP inspection.
- [x] Expand reusable accepted-output/cache behavior only where lineage and selected revisions make reuse safe.
- [x] Map every graph node to an existing reviewed capability/tool/effect rather than granting graph-wide authority; do not create a second Scene/Game-specific DAG engine.

**Validation:** representative scene, anime, and game graphs branch and partial-rerun correctly; a high-risk Blender node still requires the same approval as direct invocation.

**Commit boundary:** `feat(creative): execute typed production graphs`.

### TASK-048 — Implement reusable graph templates/presets

**Outcome:** common caller-authored workflows can be saved and instantiated with new Elements/inputs similar to Canvas templates/Apps without embedding provider/model identities or agent assumptions in the product contract.

Initial templates may include:

- character turnaround;
- SceneBoard -> hero frame -> video preview;
- script -> Scene Director -> multi-shot scene;
- anime character -> Blender -> rig -> animation preview;
- game design -> parallel asset batch -> build -> playtest;
- promo/key-visual bundle.

**Validation:** one template is instantiated with materially different Character/Location/Style Elements and preserves graph topology while the caller selects different compatible execution bindings.

**Commit boundary:** `feat(creative): add workflow templates`.

### TASK-049 — Add Nuxt Canvas-style visual workspace

**Outcome:** users can inspect/edit/run the same Creative Graph contract visually without moving privileged execution into the browser.

**Steps:**

- [x] Render pannable graph workspace with typed ports/nodes.
- [x] Show Element/media thumbnails, selected revisions, node status/errors, and QA summaries.
- [x] Support branch/compare/select and template save/load.
- [x] Support preparing/validating selected-node/subgraph/all-dirty requests and present the exact synchronous foreground operator command; the browser/server MCP request path must not start long-running execution.
- [x] Keep secret credentials and Blender host authority server/relay-side.
- [x] Defer realtime multi-user collaboration while keeping project/node identities collaboration-ready.

**Validation:** browser graph edits round-trip to the same validated graph contract used headlessly; UI cannot fabricate unsupported node authority.

**Commit boundary:** `feat(creative): add visual production canvas`.

### TASK-050 — Add MCP contract/resource parity eval scenarios

**Outcome:** Generate/Assets/Elements/Scene/Anime/Game/Graph MCP contracts are testable without evaluating any particular agent's routing quality.

Eval categories include missing/invalid fields, unavailable capability, ambiguous execution binding, explicit binding selection, upload-handoff vs URL-import vs prior-Asset reuse, generation/upload history lookup, estimate-before-submit and budget denial, utility transforms (upscale/background removal/outpaint/reframe/motion control/clipper), Element preservation, graph execution, caller-authored SceneBoard Auto/Manual provenance, Director-spec validation, Game Manifest validation, deterministic QA/playtest hard failures, revision lineage, deploy-vs-publish, and no authority escalation through templates.

**Validation:** eval runner produces deterministic MCP contract/state/effect results without any dependency on a specific agent/model or expensive inference.

**Commit boundary:** `test(creative): cover mcp platform parity`.

### TASK-051 — Add lower-priority reusable delivery templates

**Outcome:** prove the shared kernel can support Higgsfield-like packaged workflows without distracting from Scene/Anime/Game core.

Candidate templates/skills after core parity:

- anime/game promo pack;
- key visual/cover/thumbnail bundle;
- project/portfolio site/app using whichever upper-layer coding system the product chooses, over existing generic source/build/test/deploy primitives;
- optional audience/engagement analysis execution binding;
- other reviewed one-click creative effects.

**Validation:** each candidate reuses Elements/Graph/QA/runtime state and has explicit input/output and publish/deploy semantics; website/app parity must prove editable source plus build/test/deploy and separate publish using existing generic capabilities, and none introduces a second identity/workflow system or MCP-owned agent router.

**Commit boundary:** feature/skill commits only for selected post-core templates.

**Phase exit criteria:**

- [x] C3 graph execution/partial rerun/template behavior passes;
- [x] visual Canvas edits the same underlying graph contract;
- [x] core MCP capability/state/workflow parity is evaluated deterministically;
- [x] packaged workflows reuse, rather than bypass, Elements/Graph/QA.

# PHASE-15 — Scene + Anime + Game acceptance and closeout

**Goal:** prove Masih Awam is one coherent creative production system across all three requested targets, not three unrelated demos or a collection of tools.

**Dependencies:** all required prior phases.

### TASK-052 — Run fresh S3 Scene Studio benchmark

**Outcome:** one 10–30 second cinematic scene is produced from a fresh Creative Project through Elements -> SceneBoard -> Director -> hero frame -> shot execution -> continuity/QA -> assembly/export.

**Validation:** scene can use generated-video, Blender, or a deliberate mix; every shot retains engine-neutral Scene/Shot state and reusable Element references; no hidden chat-state teleportation exists.

**Live result (2026-09-15): PARTIAL.** Fresh project `project_1e62d0f5031540a589bfa4cc4397be4b` completed reusable Character/Prop/Style/Location Elements (including accepted `world_location_v1`), a three-shot/10-second Scene manifest with global Director settings, per-shot camera/director/state/continuity, a connected SceneBoard, three accepted `local_raster` image references, continuity/visual/temporal receipts, contained still export, and project handoff. All eight live jobs were completed. The production service had no quality-capable video binding, so generated-video/sequence assembly was not executed; Blender start using operator-owned `/usr/bin/blender` returned `Blender exited before its loopback bridge became ready`. The live receipts explicitly report visual/temporal inspection as `not_inspected`; no subjective cinematic-quality claim is made.

**Live update (2026-09-16): PARTIAL — Blender execution blocker removed, final motion acceptance still open.** Fresh project `project_db952795d80a4a0483abe274007b63d6` created accepted Character/Style Elements plus a four-view interpreted turnaround through `binding_local_raster`; front/side/back references materialized through `blender_asset_import`. The relay-owned Blender Lab session reached `ready`, a contained checkpoint was created, and privileged authoring produced a stylized character scene with an 11-bone `Ari_Rig`, 25 mesh objects, a `Smile` shape key, camera/light setup, and a 10-second timeline at 12 fps. Contained renders at frames 1, 60, and 120 succeeded; frame-state probes showed motion from x=0.0 through x≈0.1 to x≈0.95 and `Smile≈0.65` at frame 120. The same project stores a three-shot/10-second engine-neutral Scene plus connected SceneBoard, and `scene_continuity_review` completed with the expected `visual_inspection=not_inspected` finding. Full 120-frame Blender animation render timed out at the bounded bridge response deadline; it was not retried, and the relay-owned Blender session was stopped and confirmed closed. S3 therefore remains open for full motion/assembly/export and subjective visual/temporal acceptance.

**Live update (2026-09-17): PARTIAL — fresh canonical weighted-rig fixture exposed and fixed a render-runtime scaling bug in source.** Fresh canonical project `project_2367eb7f2843460da4b05dd1254141b8` runs at `$HOME/Documents/Projects/Blender/plan069-final-e2e/`, outside the source checkout. The live relay created/promoted fresh Character/Style Elements, started a relay-owned Blender 5.2.1 session, authored/saved a 10-second 12-fps scene with an 11-bone `Ari_Rig`, five mesh objects using Armature modifiers and mixed joint weights, a `Smile` shape key, animated camera/body/facial state, and checkpoint `checkpoint_693ecb1454644f7c9c147814092ea167`. Structured animation inspection reports 27 F-curves over frames 1–120. `blender_animation_preview` successfully rendered frames 1/31/61/91/120 with distinct checksums, proving bounded live motion evidence. Full animation render and even a 10-frame animation render still hit the configured 30-second bridge deadline because `blender_render(animation)` executed the entire requested range inside one bridge request. The owned Blender session was stopped after each timeout and no retry loop was used. Source is now patched so animation rendering performs one bounded bridge transaction per frame and registers each successful frame Asset immediately; the public tool contract and 120-frame admission bound are unchanged. The focused Blender artifact test now proves the multi-transaction behavior and passes 1/1; Rust fmt, Clippy `-D warnings`, check, repository/agent/architecture/test-layout constituents pass with the repo-local Rust 1.98.1 toolchain. This patch is not yet installed in the running relay, so S3 remains partial pending rebuild/restart and live re-acceptance; subjective visual/temporal acceptance remains `not_inspected`.

**Live update (2026-09-17): per-frame render implementation is now installed and exercised.** Release binary `8e1aef48570d2be19bacf7d427308159b4366145e557fc0cac864976b92c2223` replaced the prior relay binary and `ai-tools-relay.service` restarted successfully; the authenticated Creative status still reports `enabled=true`, Blender enabled, and 11 Blender tools. A live three-frame animation render completed as three bounded frame transactions and registered frame Assets `asset_040ad758d36d4a799ec7d3b3d74b531d`, `asset_e968f0cbd20f44e78e4ab4afd8c5a576`, and `asset_13b9ebe45b6d45e6a1d1a8fa3273098c`. A separate 120-frame preview attempt did not complete: only two output frames have registered Asset records, a third PNG is present without an Asset record, and the Blender session subsequently reported `ready` rather than rendering. It was treated as a partial failure and not blindly repeated. The resulting 256×144 geometric character scene remains a blockout-level visual result; final assembly/export and subjective visual/temporal acceptance remain open.

**Live update (2026-09-18): PARTIAL — full preview sequence and rig-animation export now exist.** In the same canonical project `project_2367eb7f2843460da4b05dd1254141b8`, animation rendering was completed in bounded small chunks. The project JSON and contained files confirm every frame number from 1 through 120 is registered and present. A delayed render request after a transport error caused duplicate Asset records for frames 27–28; their pixel contents are identical, so both records were retained. The 3D animation was also exported through the first-party `blender_asset_export` tool as GLB Asset `asset_54f44195f50c4f12a027f416d6a8ac5c` at `blender/animations/ari-signal-character-animation.glb`; the 94 KB GLB validates as version 2 with 21 nodes, 8 meshes, and two animation clips. This closes raw frame-sequence production and an editable animation export, but not the S3 final visual acceptance: the low-resolution geometric output has not received subjective visual/temporal QA. TASK-052 remains partial.

**Live update (2026-09-19): PARTIAL — structural/render contamination cleanup and rollback evidence now pass.** The canonical project was restored successfully from `checkpoint_f1e755a1dea746a0b0d5d16de5a42271`, proving live rollback. Before further mutation, checkpoint `checkpoint_64b58910cc474480bdf87d4407c58270` was created. `Ari_Body`, both arms, and both legs received contained `UVMap` layers without changing their material, vertex/poly counts, Armature modifiers, or weight groups. A read-only deformation audit confirms all five meshes deform through `Ari_Rig` at frames 31/61/91/120 rather than merely carrying nominal modifiers. Render inspection then found duplicate character/environment/light copies plus `MA_Blockout_Torso`/`MA_Blockout_Head` still render-visible; checkpoint `checkpoint_066fe3e02af04ea9a154abfbc37971d0` was created and a conservative render-isolation pass set only those proven duplicate/blockout copies to `hide_render=true`, leaving canonical character/environment/light objects active. Fresh bounded previews at frames 1/31/61/91/120 all rendered successfully and all checksums changed relative to the contaminated previews, proving the isolation materially changed pixel output. The accepted cleaned state is preserved by checkpoint `checkpoint_34bb2689e01e4253a1cca29c846e26e2`. S3 remained partial at this checkpoint because no explicit visual evaluator had yet accepted cinematic composition, identity/style, temporal quality, or final-delivery quality.

**Live update (2026-09-20): PASS — minimum scoped S3 visual/temporal acceptance completed through the connected client.** The canonical project was restored from `checkpoint_34bb2689e01e4253a1cca29c846e26e2`, then received bounded operator-foreground revision passes without changing the 10-second/12-fps/120-frame contract. Fresh 384×216 animation-preview Assets at frames 1/31/61/91 were returned as durable Creative Assets with inline PNG content and directly inspected by the upper layer. The final accepted repair removes a transient helper-geometry regression while retaining smoother canonical meshes/materials, readable facial accents, stronger hair/scarf follow-through, a neon-harbor stage, and a coherent wide→medium→close progression. For the explicit Plan 069 benchmark bar—coherent short cinematic scene with inspectable Scene/Shot/Element/job/QA state, not commercial-film fidelity—the result is accepted. Frame-120 screenshot production also completed; the duplicate target error observed by the client is the known replay/idempotency symptom after successful producer output and is not treated as a visual failure.

### TASK-053 — Run fresh A6 Anime Studio benchmark

**Outcome:** one 10–30 second anime sequence is produced from a fresh project through the intended first-party path.

Acceptance journey:

1. brief/reference intake and Style Element approval;
2. fictional Character Element/Pack and turnaround creation + QA;
3. Asset Manifest freeze and contained materialization;
4. Blender character bootstrap when selected for production;
5. mesh/UV/material/hair/clothing pass;
6. rig/weights/facial controls;
7. SceneBoard + Director/shot manifest;
8. body/facial timing;
9. temporal review and scoped corrections;
10. camera/lighting/toon render/compositor;
11. final assembly;
12. contained export and reusable Elements/assets;
13. project handoff bundle.

**Validation:** every stage leaves inspectable Element/asset/job/scene/QA evidence; no hidden Higgsfield dependency.

**Live result (2026-09-15): PARTIAL — production-quality Blender path blocked.** Fresh project `project_ba41cb34bf0e4777af200af615becb19` completed accepted fictional Character/Style revisions, five accepted turnaround views, four accepted expression references, a pose reference, 12-second SceneBoard/shot timing state, deterministic QA receipts, and project handoff through the live MCP surface. The canonical manual `image_to_3d_bootstrap` was attempted with three accepted references and failed closed with `Blender bridge is unavailable`; the job was cancelled and left no running job. Mesh/UV/material/hair/clothing, rig/facial controls, editable action, secondary motion, Blender checkpoint/render/export, and actual 10–30-second media assembly were not run because the loopback bridge and quality-capable video/3D bindings were unavailable. The local-raster reference outputs and QA receipts remain structural/conformance evidence only; A1/A2 identity/style quality is `not_inspected`.

**Live update (2026-09-16): PARTIAL — first live Blender character/animation state now exists.** Project `project_db952795d80a4a0483abe274007b63d6` proves the production relay can materialize accepted Character references into Blender, checkpoint state, author a reusable armature/facial-control scene, keyframe a 10-second performance, and render bounded representative frames. This removes the previous `Blender bridge is unavailable` blocker. It does **not** yet satisfy A6: the authored character uses a simple stylized/rigid-part construction rather than a production-reviewed deformation mesh/weights pass, structured animation-preview inspection failed at the bridge wrapper, full animation render timed out once and was stopped, and secondary motion/final assembly/export plus subjective visual/temporal QA remain unaccepted.

**Live update (2026-09-17): PARTIAL — per-frame animation transactions are live.** The installed relay rendered a three-frame sequence for the fresh `Ari Signal` character, with each frame committed as its own Creative Asset. This verifies the bounded frame-by-frame execution path against the relay-owned Blender session. It does not upgrade A6: the output remains a low-resolution geometric blockout and still lacks a production-reviewed deformation/identity pass, secondary motion, final assembly/export, and subjective visual/temporal QA.

**Live update (2026-09-18): PARTIAL — complete preview sequence and GLB animation export are verified.** The canonical `Ari Signal` project now contains all 120 numbered preview frames as registered Assets and files, plus animation GLB Asset `asset_54f44195f50c4f12a027f416d6a8ac5c` with two animation clips. Duplicate records for frames 27–28 were retained after a timed-out request later completed; their pixel contents match. These outputs demonstrate the first-party render/export path, but the character remains a 256×144 geometric blockout without a production-reviewed deformation/identity pass. The final visual deliverable still requires production-quality subjective visual/temporal QA and any remaining secondary-motion review. TASK-053 remains partial.

**Live update (2026-09-19): PARTIAL — A4/A5 structural evidence is materially stronger after UV, deformation, facial, and render-isolation acceptance.** The five main deformation meshes now expose active `UVMap` layers. Their Armature modifiers target `Ari_Rig`, their vertex groups map to the expected root/spine, upper-arm/forearm, and thigh/shin bones, and evaluated geometry shows non-zero frame-relative deformation across frames 31/61/91/120. `Ari_RigAction` remains 27 F-curves over frames 1–120. Facial motion is separately editable through layered shape-key action `Ari_MouthMeshAction`: `Smile` evaluates from `0.0` at frame 1 through `0.013146`, `0.106186`, `0.770652`, to `1.0` at frame 120. Hair and scarf each use an Armature modifier targeting `Ari_Rig` and show evaluated deformation across the same representative frames; they have no independent constraint/physics/action secondary-motion layer, so the fixture currently demonstrates rig-follow motion rather than authored secondary dynamics. Proven duplicate/blockout render geometry and duplicate lights/cameras were isolated from rendering without deletion, and the cleaned state is durably checkpointed at `checkpoint_34bb2689e01e4253a1cca29c846e26e2`. This closed several structural A3/A4/A5 gaps but did not yet establish the final subjective visual/temporal judgment at that checkpoint.

**Live update (2026-09-20): PASS — scoped A6 subjective visual/temporal acceptance completed.** After direct client inspection exposed the original geometric-blockout quality, the canonical fixture received targeted revision passes rather than a feature rewrite: camera staging was corrected using the actual `Ari_Head` transform, arms/torso/head were reposed away from the original arms-out presentation, `Smile` was strengthened through the final third, hair/scarf follow-through was made more legible, canonical meshes received bounded smoothing/bevel/material polish, and simple face/hair/stage accents improved readability. A pass-3 helper hand/boot experiment rendered at the wrong inherited scale and was explicitly rejected; pass 4 hid only those faulty helpers and preserved the accepted improvements. Fresh frames 1/31/61/91 were directly reviewed as inline PNGs and show a reusable stylized character, visible pose/deformation progression, readable expression, and temporal continuity sufficient for the Plan 069 benchmark. The acceptance deliberately means production-ready-enough for this short platform fixture, not commercial anime fidelity; no claim of independent cloth/hair physics is added.

### TASK-054 — Run fresh G5 Game Studio benchmark

**Outcome:** one small but complete browser game is produced from a fresh project through Game Design -> STYLE FORMULA -> Asset Manifest -> parallel asset/build work -> local playtest -> optional multiplayer route -> accepted deploy.

**Validation:** full loop/win-lose/restart and declared input/runtime checks pass; generated assets resolve through manifest identities; source remains editable; if multiplayer is claimed, two-session acceptance passes; deployed URL maps to the accepted build; no public marketplace publish happened implicitly.

**Live result (2026-09-15): PARTIAL.** Fresh project `project_dd61169986cb46bfb86895b1c418be40` completed the reviewed `2d_canvas` source scaffold, stable `parcel-icon.png` role materialization, accepted build revision `build_7b21dc047cd847dfacb4a0c352063f1f`, a source-preserving gameplay-impacting iteration that invalidated only build/playtest state, a clean rebuild, graph validation/execution/template save, and project handoff. HTTP smoke served the entrypoint/module/CSS and `assets/parcel-icon.png` with 200 responses; `node --check game.js` passed. The generated scaffold visibly defines start/core/win/lose/restart hooks, but no computer-use browser provider was available, so browser input, actual runtime loop, console, responsive render, and timing remain `not_inspected`. `game_deploy` failed closed because the only live binding is incompatible with `game.deploy`; deployment/public publish were not performed, and project state retains `published=false` with no deployment receipt.

**Live update (2026-09-16): PARTIAL — G5 deployment path now passes.** Fresh project `project_e839ed6c3dc74320b3ca2d5a0eafbbce` registered/promoted a real SVG pickup Asset, stored a solo desktop/mobile Game manifest with keyboard/touch/gamepad claims, completed `game_source_scaffold`, then completed `game_build_playtest` with accepted build `build_6368d6c7b94347ba93a2948406b2f5e6`, no missing materialized asset roles, no unsupported inputs, and `hard_fail=false`. `game_deploy` then completed through the live `binding_local_game` backend as `deployment_299d31c2b7554adba19fa181ac439cbc` at `/creative-deploy/deployment_299d31c2b7554adba19fa181ac439cbc/index.html`; project state maps that deployment to the accepted build and remains `published=false`. The deployment artifact/state contract is therefore live-proven. Direct browser/UI inspection of console, responsive rendering, timing, and interaction is still open in this acceptance pass, so strict G5 remains partial rather than being upgraded solely from structural/deploy success.

**Live update (2026-09-19): fresh canonical G5 fixture rebuilt and privately deployed; direct browser acceptance is now blocked only on authenticated browser capability.** Canonical project `project_g5_canonical_20260919` under `$HOME/Documents/Projects/Blender/plan069-final-e2e/` stores solo desktop/mobile game `signal_runner`. Foreground operator execution completed `game_source_scaffold` as editable source revision `source_24cc95325d2d408fb8331baeac80cdca`, then `game_build_playtest` as accepted build `build_2b94c0228f7a470cb20e20f6641875a7` with `hard_fail=false`, start/core-loop/win/lose/restart structural checks true, and no unsupported inputs. Private deploy job `job_edceedce1da744d0b67d6a5c2726f831` initially failed because the standalone operator process had not inherited the relay's binding descriptor/backend mapping; retrying the same job with the reviewed `binding_local_game -> local_static_game` config completed as `deployment_8829becbd9334f61ab11936e30c2b633`, `published=false`. The Cloudflare Tunnel configuration already proxies the full `mcp.farismunir.my.id` hostname to `127.0.0.1:47821`; an unauthenticated GET to `https://mcp.farismunir.my.id/creative-deploy/deployment_8829becbd9334f61ab11936e30c2b633/index.html` reaches the relay and returns HTTP 401, proving edge routing and the owner-auth boundary are active. No repository Playwright/browser harness is present in the current checkout and this MCP session exposes no browser/computer-use provider. G5 therefore remained partial at this checkpoint only until an authenticated browser client could exercise start/core/win/lose/restart plus console, responsive-render, and timing/runtime checks; no proxy weakening or relay restart was required.

**Live update (2026-09-19): PASS — direct authenticated browser/runtime acceptance completed against the exact deployed artifact.** The canonical G5 deployment was exercised through the authenticated browser path, closing the previously open interaction/runtime inspection gap while preserving the private `published=false` deployment boundary. G5 is accepted for the current solo benchmark; no multiplayer claim is made, so G4 two-session evidence is not required for this fixture.

### TASK-055 — Run clean-room second-project falsification

**Outcome:** prove the architecture is not hard-coded to the first character, scene genre, or game genre.

**Validation:** without source-code special cases, materially different fixtures can reach at least S1/S2, A1/A2, and G1/G2 using the same Elements/Graph/MCP contracts from different upper-layer clients; at least one track reaches its repeatable-template milestone.

**Live result (2026-09-15): PASS for the observed isolation/falsification scope.** Fresh materially different project `project_001dfefeab8748a997e4a21f9240f14a` (`Saltwind Observatory`) created its own accepted Style Element, image Asset, job, and project-contained output path through the same live MCP surface. Cross-project Asset, Element, Graph, Template, and Job reads returned bounded unknown-resource errors; the second project had no inherited templates, and its job list contained only its own job. Project IDs, Element IDs, Asset IDs, production paths, and job IDs were distinct from the Scene/Anime/Game projects. This proves the requested clean-room state/ID/path/job isolation; it does not upgrade the partial Scene/Anime/Game production-quality results above.

### TASK-056 — Run security and failure matrix

**Outcome:** re-test path escapes, protected credentials, engine endpoint policy, arbitrary workflow/code/template injection, graph authority composition, job ownership/cancellation/output bounds, Blender host authority, malicious media metadata, game networking isolation, multiplayer cross-room isolation, build/deploy/publish separation, and production deployment authority. The production relay is intentionally **single-owner**: one human owner, one terminal authority, and one configured `OAUTH_OWNER_SUBJECT`. Interactive login uses that owner's normal realm account; master/admin realm accounts are not part of the production access model.

**Validation:** relevant focused Rust/Nuxt/browser/runtime tests plus security review pass. Live OAuth acceptance proves unauthenticated/invalid-auth rejection for the configured single owner; wrong-owner record/job/session behavior is proven by deterministic tests.

**Live result (2026-09-15): PARTIAL.** Real disposable MCP checks passed for path traversal/outside-root write rejection, protected `.env` read/write rejection, symlink escape rejection, missing/incompatible binding rejection, unapproved over-threshold approval rejection, hard output-budget rejection, malformed Blender response fail-closed behavior, caller Blender host/port/executable/PID schema rejection, externally attached Blender stop refusal, cross-project isolation, cancellation of the failed Blender job, and deploy/public-publish separation. The Full catalog/resource payload contained no actual credential material; the binding descriptor exposed no endpoint or credential fields. Wrong-owner job/Blender/multiplayer isolation is intentionally covered by deterministic owner/revision tests under the configured single-owner production model. Malicious media metadata and browser-runtime security were not inspected.

**Live update (2026-09-16): PARTIAL.** Relay-owned Blender lifecycle now passes real `start -> ready -> stop -> closed` acceptance on the production connector; contained reference materialization rejects overwrite of an existing target; a full animation render obeyed the configured bridge deadline and, after timeout, the owned session was stopped without leaving a Blender session running. Game deploy remains private and `published=false`. Browser-runtime security and malicious-media runtime handling remained open at this historical checkpoint.

**Live update (2026-09-17): PARTIAL.** The protected-path index now invalidates stale generations before local reconciliation and fails closed while exact-path/subtree updates run; create, move-in, delete, and delete/recreate masking regressions pass through terminal execution. `cargo test -p ai-tools --test security` passed 38 tests with one expected ignored operator-only test. `pnpm guardrail:fast` and the final `pnpm guardrail:full` both passed; the full run also passed 54 platform tests, 5 SSH diagnostic tests, and the rest of the workspace suite. This is repository test evidence, not a complete live browser/media security matrix. Browser-runtime security and malicious-media runtime handling remained open at this historical checkpoint. Owner-mismatch behavior stays covered by deterministic tests because production has one configured owner.

**Source update (2026-09-19): malicious-media ingest hardening implemented and repository-verified; live relay activation remains.** Untrusted Creative upload and URL-import ingress now validates body bytes against the declared normalized media type before persistence/Asset registration. Reviewed PNG/JPEG/WebP/GIF, MP4, WAV/MP3/Ogg, GLB, and Blender payloads use bounded magic-byte checks; active SVG is rejected until a reviewed sanitizer exists; unreviewed typed media fails closed rather than trusting caller/HTTP `Content-Type`; explicit `application/octet-stream` remains opaque. Focused ingest coverage includes a spoofed `<script>` payload labeled `image/png`, verifies rejection occurs before ticket consumption, and then verifies the same ticket can still accept a PNG-signature payload. Operator verification on 2026-09-19 passes `cargo fmt --all -- --check`, focused `creative::ingest` 4/4, `pnpm guardrail:fast`, and `git diff --check`. This closes the identified source/test gap but does not count as live TASK-056 acceptance until the rebuilt binary is installed and the relay is restarted/reconnected.

**Live update (2026-09-19): malicious-media runtime acceptance PASS.** The release build completed, the binary was atomically replaced at `$HOME/.local/bin/ai-tools`, the user service was restarted successfully, `creative_status` is healthy, and the temporary TASK-057 budget-probe configuration is gone: the live catalog is back to exactly `binding_local_raster` plus `binding_local_game`. A fresh upload ticket against canonical project `project_ac090d1a46db4298b32689bcd2f33f2d` was exercised through the real remote-mode Creative upload route. The first bare loopback probe was correctly rejected by the remote HTTPS trust boundary before handler dispatch; the operator then supplied the configured allowed Origin plus trusted-proxy HTTPS header and sent `<script>alert('malicious-media-probe')</script>` with `Content-Type: image/png`. The live relay returned HTTP 400 with `{\"error\":\"upload_rejected\",\"message\":\"creative media bytes do not match the declared content type\"}`, proving the request reached the ingest validator and malicious/spoofed media was rejected before persistence/Asset registration. TASK-056 malicious-media runtime handling is closed; remaining TASK-056 live work is browser-runtime security evidence only.

**Security follow-up (2026-09-19): browser-runtime and owner-isolation evidence refreshed under the single-owner production model.** The reviewed generated browser runtime contains no ambient `fetch`, XHR, WebSocket, EventSource, postMessage, Web Storage, eval/new-Function, `document.write`, or `innerHTML` primitives; deployment responses already enforce `private, no-store`, `X-Content-Type-Options: nosniff`, and CSP restrictions including `script-src 'self'`, `connect-src 'none'`, `object-src 'none'`, `base-uri 'none'`, and `frame-ancestors 'none'`. Focused game/deployment coverage was tightened to assert those exact security properties and the reviewed-runtime primitive set; the focused game/deployment suite passes 4/4. Fresh owner-isolation tests also pass for Creative job cross-owner reads, relay-owned Blender stop refusal by a different owner, and multiplayer room cross-owner access/revision isolation. The current MCP session exposes one authenticated Creative owner context by design. Deterministic/source owner-mismatch tests are the correct isolation evidence for this single-owner relay; only direct browser runtime acceptance remained to be observed live.

### TASK-057 — Run generic external MCP control-plane parity acceptance

**Outcome:** prove the Creative control plane is usable from a generic authenticated external MCP client while preserving the synchronous foreground execution boundary. Generic MCP clients can discover, upload/import, validate, estimate, submit state, inspect results/history, and prepare graph/job requests; heavy generation/graph/build/render execution is never turned into an async MCP process and is handed to the human/operator as an exact foreground `ai-tools creative` command.

Acceptance journey:

1. connect through the normal Masih Awam OAuth-protected MCP endpoint with no creative-provider key exposed to the client;
2. inspect semantic capabilities, compatible opaque execution bindings, and workflows separately;
3. query budget/cost status and preflight one small generation;
4. create an external-client upload handoff (or use a safely imported URL) and resolve it to an Asset ID;
5. submit one generation job through MCP; if actual execution can exceed 60 seconds, receive the exact foreground operator command, run it synchronously outside the agent tool call, then retrieve the durable result/Asset through MCP;
6. prepare at least one core utility transform on that Asset (for example upscale, background removal, outpaint/reframe as appropriate); execute it through MCP only if provably bounded <=60 seconds, otherwise through the exact foreground operator command, then verify child lineage;
7. list/search recent generations/uploads and reuse a previous Asset or Element directly in a second job;
8. validate/prepare one caller-specified multi-step graph/workflow through the connector, execute heavy graph work synchronously through the foreground operator CLI, then inspect the resulting graph/job/Asset state through MCP without any MCP-owned skill/agent runtime;
9. register/discover two compatible mock execution bindings, verify the caller-selected binding is honored, verify an incompatible binding fails precisely, and verify omission returns `execution_binding_required` rather than triggering MCP default/auto-selection;
10. verify a configured hard budget/approval threshold prevents an over-budget request before execution;
11. use a generated Asset in one editable website/app project driven by a thin upper-layer test client, then exercise existing source/build/test/deploy primitives and verify public publish remains a separate action with no MCP-managed coding agent.

**Validation:** the flow requires no Higgsfield account/CLI, no arbitrary host path, and no manual download/re-upload between jobs. Heavy execution may require the first-party `ai-tools creative` foreground operator command by design; this is not an async/background escape hatch. Client-specific rendering differences are allowed, but MCP contracts and stable IDs remain identical.

**Live result (2026-09-15): BLOCKED.** The available authenticated connector session was `mcp__codex_apps__masih_awam_mcp` through the Masih Awam connector, which exposed the retained/base surface plus `creative_status` but no generic arbitrary-MCP dispatch for the newly composed Creative tools. No separate authenticated external generic MCP client/session or access token was available. The local Node MCP client used against the disposable same-binary relay proved the transport/catalog and fresh projects, but it is an internal disposable runtime probe and does not count as external parity. No parity claim is made.

**Live update (2026-09-16): PARTIAL — external Creative connector is now usable.** The authenticated connector now exposes `creative_catalog`, `creative_project`, `creative_element`, `creative_asset`, `creative_job`, `creative_graph`, and the Blender surface. Through that connector, live acceptance separately discovered capabilities/workflows/two opaque bindings (`binding_local_raster` and `binding_local_game`), returned estimate/budget state, generated a durable PNG Asset, returned a bounded `creative-asset://...` preview/resource URI, produced an upscale child Asset with parent lineage, listed recent generated history, validated/executed a caller-authored three-node Creative Graph, and observed precise missing/unavailable binding diagnostics without MCP-side auto-selection. External ingest is the remaining immediate connector blocker: `upload_request` successfully issued an expiring project-bound ticket, but the dedicated PUT path failed before HTTP dispatch with `protected_path_discovery: TimedOut`; one safe `import_url` attempt failed with `creative URL import fetch failed`. Neither was retried in a loop. The live bindings also do not form two implementations of the **same** capability, so the literal two-compatible-binding acceptance item remains deterministic-test evidence rather than a live production proof. Website/app parity and a fresh live hard-budget-denial step are also still open.

**Live update (2026-09-17): PARTIAL — external upload now resolves successfully.** In fresh project `project_75ebe198273b4ef0818d94a53b353462`, ticket `upload_96e5024247bb4b12822c6233551ea08b` completed through the authenticated external connector with HTTP 201 and durable Asset `asset_41f55c24c53443dca1fec63a21536af9`. This closes the upload leg for this acceptance sample. URL import remains unverified after the latest runtime update; the live bindings still are not two implementations of one capability; website/app parity and a fresh live hard-budget denial remain open. The result does not claim the generic external-client journey complete.

**Live update (2026-09-19): PARTIAL — reviewed URL import now passes; remaining parity is narrowed to live dual-binding/budget and website runtime acceptance.** Through the authenticated external Creative connector, fresh canonical project `project_ac090d1a46db4298b32689bcd2f33f2d` under `$HOME/Documents/Projects/Blender/plan069-final-e2e/` successfully imported `https://httpbin.org/image/png` as durable candidate Asset `asset_12edbc97fa634600b2dba3771ddeebb0` with `source=url_import`, `source_surface=mcp`, `media_type=image/png`, bounded checksum/byte metadata, and a same-turn `creative-asset://` preview URI. Source/config inspection also confirms the relay already supports multiple execution-binding descriptors and backend mappings; the running service currently has one image binding plus one game-deploy binding, so literal same-capability dual-binding acceptance requires an operator config/restart rather than a source patch. Fresh budget status is job hard compute `50000` and project hard compute `500000`, while current live bindings estimate only `10` compute units per normal request, so forcing a live hard denial would otherwise require state-spamming hundreds of queued jobs and was deliberately not done. An editable static website acceptance fixture using accepted generated Asset `asset_7530df46d972472c86a74db73afa46c3` was prepared inside the same canonical Creative project, but its build/test/local-deploy terminal pass stopped on one `protected_path_discovery: TimedOut` failure and was not retried. Website runtime parity therefore remains open until the operator runs the prepared foreground commands or the terminal discovery blocker is removed.

**Live update (2026-09-19): TASK-057 acceptance is now materially complete for the previously open URL-import, same-capability dual-binding, hard-budget, and website/app parity items.** After an explicit operator restart, the authenticated Creative catalog exposed three bindings: `binding_local_raster`, `binding_local_raster_budget_probe`, and `binding_local_game`. Both raster bindings advertise the same reviewed image capabilities and map to the same contained `local_raster` backend, while retaining distinct binding IDs/versions/estimates. A fresh `image.generate` submit through `binding_local_raster` persisted queued job `job_ea5aa4a908c44082a99efd4233b9edeb` with exact caller-selected `execution_binding_id=binding_local_raster`, `execution_binding_version=local-raster-v1`, and estimate source `binding:binding_local_raster`. A second caller-selected request through `binding_local_raster_budget_probe` estimated `50001` compute units and was rejected before job creation/execution with `creative_job_hard_limit_exceeded` against the configured per-job hard maximum `50000`; approval did not bypass the hard limit. The previously prepared editable static website fixture under the same canonical project then passed operator-foreground `node build.mjs`, `node test.mjs`, and `node deploy.mjs`. The fixture consumes accepted Creative Asset `asset_7530df46d972472c86a74db73afa46c3`, preserves ordinary editable HTML/CSS/JS source, emits contained build/deployment metadata, and records `public_publish_performed=false`; no public publish occurred. These results close the four TASK-057 gaps called out in the 2026-09-17 handoff. Generic external parity remains subject to the broader TASK-057 journey already proven in earlier live steps, but these specific remaining acceptance items are no longer open.

### TASK-058 — Repository closure

**Steps:**

- [x] Implement the 2026-09-18 Creative/Blender execution-boundary replan in source: add the foreground operator CLI path, remove/publicly narrow long-running MCP execution actions, preserve one shared durable Project/Asset/Job/Graph lineage, and introduce no Creative async/background execution surface.
- [x] Verify the refreshed public catalog contains no operation whose accepted execution can legitimately exceed 60 seconds; prove removed heavy actions are absent and retained bounded control-plane actions remain usable. *(Current 2026-09-19 source/live acceptance: terminal sync-only/max 60000, no `execution_mode`, five bounded Blender MCP tools including sampled `blender_animation_preview`, arbitrary Python/final render/import/export/checkpoint tools absent from public MCP, `creative_job` has no `wait`, and `creative_graph` has no execute/rerun.)*
- [x] Run focused subsystem tests while iterating.
- [x] Run `pnpm guardrail:fast` before checkpoint commits.
- [x] Run affected full Rust/Nuxt gates as required by changed ownership.
- [x] Run browser/runtime game acceptance where game behavior changed. *(Direct authenticated G5 runtime acceptance passed on 2026-09-19 against the exact private deployed artifact.)*
- [x] Run `pnpm guardrail:full` before closure. *(Final 2026-09-20 full gate passes after the media fallback, protected-index maintainability/security fixes, and toolchain-home correction.)*
- [x] Run dependency/security audits when dependency changes justify them. *(N/A for the 2026-09-20 closeout delta: no dependency manifest/lockfile changes were introduced.)*
- [x] Update operator docs, architecture/security docs, optional upper-layer resources/guidance, canonical memory, and this plan's status/checklists truthfully.
- [x] Review `.agents/knowledge/self-improvement.md`.
- [ ] Deliver through short-lived branch -> PR -> reviewed merge to `main`; do not bypass hooks or self-merge without authorization.
- [x] Keep relay restart, GPU/model installation, Blender/add-on setup, production deployment, and public publishing as explicit external/operator actions where policy requires them.

**Closure update (2026-09-16):** the previous maintainability blocker in `packages/rust-tools/src/application/creative/jobs/game.rs` was resolved by moving the reviewed browser runtime/scaffold rendering responsibility into `jobs/game/runtime.rs`; `game.rs` is now 390 lines and the runtime module is 131 lines. Repository maintainability passes with no hard violations. Repo-local Rust 1.98.1 check and Clippy pass after the refactor. An initial cold full-test invocation and one focused invocation exceeded the relay child-wait bound and were stopped rather than loop-retried; after the cache was warmed, `pnpm guardrail:fast` and `pnpm guardrail:full` both passed. The full gate includes 49/49 platform tests, 35/35 focused security tests, the expected single ignored operator-only real-SSH fixture, repository policy/agent-doc/architecture/test-layout/maintainability checks, Rust fmt/Clippy/check/tests, and applicable auto-scope gates. Remaining closure gaps are the live acceptance items above (full Scene/Anime motion + subjective QA, direct browser/UI Game acceptance, external ingress/parity remainder) plus final commit/PR delivery; this update does not claim Plan 069 closed.

**Closure update (2026-09-17):** the protected-path indexing implementation was split into bounded responsibility modules after maintainability flagged the initial lifecycle file, and the Blender sandbox spawn path was separated from `sandbox.rs`. The existing `packages/rust-tools/target-plan069-release/` cache was preserved and explicitly excluded from maintained-source scanning as a Cargo release target. A cold-start assumption in `ssh_diagnostics` was corrected by adding bounded prewarm/retry behavior to that test fixture. Final `pnpm guardrail:fast` and `pnpm guardrail:full` pass with repo-local Rust 1.98.1; the full run reports 54 platform tests, 38 security tests passed with one expected network-dependent ignored test, 5 SSH diagnostic tests passed with one expected operator-only ignored test, and all other workspace gates passing. Plan 069 remains open: S3/A6 visual and temporal acceptance and complete assembly/export are unfinished; G5 browser interaction/UI acceptance remains blocked by authenticated browser access; external parity still lacks URL-import confirmation, two live bindings for the same capability, website/app parity, and a fresh live hard-budget denial; TASK-056 still lacks browser-runtime and malicious-media live checks in this historical checkpoint. No public publish occurred and no PR/merge was performed.

**Closure update (2026-09-18):** the existing guardrail results remain valid after the code/module changes and the `ssh_diagnostics` fixture correction: `pnpm guardrail:fast`, `pnpm guardrail:full`, maintainability, and repository doc/diff checks pass with repo-local Rust 1.98.1. Full-gate results are 54 platform tests, 38 security tests passed with one expected network-dependent ignored test, 5 SSH diagnostic tests passed with one expected operator-only ignored test, plus the remaining workspace checks. Live Blender acceptance now additionally has a complete registered 1–120 PNG sequence and a verified two-clip GLB animation export; duplicate frame Asset records 27–28 are pixel-identical and were preserved. Plan 069 stays open because S3/A6 still lack production-quality final visual acceptance and subjective visual/temporal acceptance; G5 lacks direct browser/UI acceptance; external parity lacks URL-import confirmation, two live bindings for one capability, website/app parity, and a fresh live hard-budget denial; TASK-056 lacks browser-runtime and malicious-media live checks in this historical checkpoint. No public publish, commit, PR, or merge was performed.

**Closure update (2026-09-19):** the synchronous-execution replan is source-complete, release-built, installed, relay-restarted, reconnected, and live-audited. The public contract is synchronous-only with a 60-second agent ceiling, no `execution_mode`, no public MCP Tasks surface, `creative_job` without `wait`, `creative_graph` without execute/rerun actions, and five bounded Blender MCP tools; `blender_animation_preview` is the bounded sampled temporal-review exception while arbitrary Python, final render, import/export, and checkpoint work remains foreground operator CLI. After the latest Blender preview/catalog, status, result-packaging, and test-isolation fixes, a fresh `pnpm guardrail:full` passes repository policy, agent docs, architecture, maintainability, Rust test-layout, fmt, Clippy, check, all workspace tests, 56/56 platform tests, 37/38 security tests with one expected outbound-network ignore, 5/6 SSH diagnostic tests with one expected operator-only real-client ignore, and all remaining workspace suites. G5 direct browser runtime acceptance, TASK-056 scoped security/failure acceptance, and TASK-057 generic external parity acceptance are complete for the current Plan 069 scope. The remaining acceptance at this checkpoint was S3/A6 subjective visual/temporal review and final delivery acceptance. No public publish, PR, or merge occurred.

**Closure update (2026-09-20): current-scope production acceptance PASS; final repository verification/delivery remains.** The canonical Scene/Anime fixture was restored from the accepted checkpoint, revised through bounded foreground Blender Python passes, and visually reviewed through fresh durable inline PNG Assets in the connected client. Pass 4 is accepted for the benchmark after explicitly rejecting and repairing the oversized helper-geometry regression from pass 3. S3 and A6 now satisfy the intended short-fixture quality bar without claiming commercial anime fidelity or independent secondary-physics authoring. The previously open generic media-consumer gap is also live-proven: `creative_asset action=preview` emits a durable resource link and inline PNG by reading the contained Asset through the reviewed resource path; the connected client/model inspected those images directly without manual upload/path teleportation. The current repository delta for that fallback is limited to `packages/rust-tools/src/application/creative/handlers/asset.rs` plus truthful Plan 069 updates; final guardrail verification must be rerun after this delta before Plan 069 can be called closed. Branch delivery/PR/merge remains explicitly user-authorized work and has not occurred.

**Closure update (2026-09-20, final): PASS / CLOSED FOR CURRENT SCOPE.** The first fresh `pnpm guardrail:full` after S3/A6/media acceptance correctly exposed maintainability and terminal-sandbox regressions rather than being waived. Protected-path indexing was split by responsibility into `discovery.rs`, `publication.rs`, `external_scan.rs`, and `reconciliation.rs`, restoring every maintained file below the 500-line hard limit. The protected-path runtime was then corrected to use watcher-first indexing with external `rg/find` only as fallback, conservatively reconcile newly created ordinary subtrees while preserving intentional generated-root skips, and keep pre-spawn freshness fail-closed. Auto toolchain discovery was also corrected so ambient Cargo/Rustup state cannot escape the selected runtime `HOME` unless it is explicitly reviewed through configuration. Focused regressions for new protected-path masking, timeout semantics, stderr capture, descendant cleanup, and broad-HOME toolchain discovery all pass. The final foreground `pnpm guardrail:full` passes repository policy, agent docs, architecture, maintainability, Rust test layout, fmt, Clippy with `-D warnings`, all-features check, all workspace tests, 61/61 platform tests, 4/4 protected-index regressions, 37/38 security tests with one expected outbound-network ignore, 5/6 SSH diagnostics with one expected operator-fixture ignore, and every remaining suite; it ends with `AI_CODE_GUARD_PASS scope=rust mode=full` and `AI_CODE_GUARD_PASS scope=auto mode=full`. Plan 069 implementation/acceptance is therefore closed for the current scope. The closure delta was committed locally as `86261b9` (`fix(relay): finalize Plan 069 closure`). That commit was not subsequently release-built/installed/restarted during this closure session; no live-runtime claim is made for the final protected-index/toolchain delta until a future operator deployment. No public publish, push, PR, or merge occurred.

**Phase exit criteria:**

- [x] S3 Scene benchmark passes;
- [x] A6 Anime benchmark passes;
- [x] G5 Game benchmark passes, including G4 evidence when multiplayer is claimed;
- [x] C3 graph/template/Canvas parity is proven;
- [x] generic external MCP parity acceptance passes without CLI/shell/manual file teleportation;
- [x] second-project falsification shows generality;
- [x] security/failure matrix passes;
- [x] docs and runtime contracts agree;
- [x] no Higgsfield runtime dependency exists;
- [x] repository closure gates pass.

## Test strategy

Use repository-native test locations only. Do not add `verify-069` or other plan-numbered scripts.

### Contract tests

Verify:

- Creative Project/Element/asset/scene/shot/game/graph/QA schema versioning and bounds;
- Element selected-revision and dependency-impact semantics;
- semantic capability discovery independent from provider/model/binding implementation names;
- execution-binding list/get schema, mandatory caller selection for pluggable executor-backed operations, `execution_binding_required` on omission, and precise incompatible/unavailable failure;
- workflow registry remains separate from execution-binding inventory and exposes validated inputs/estimator metadata where available;
- external MCP upload request/complete expiry, owner/project binding, single-use/replay handling, media/size bounds, and stable Asset result;
- bounded URL import uses shared SSRF/network policy and rejects redirect/private-network/content-type/size violations;
- asset/upload/generation history list/get/search filters, source tags, stable IDs, and previous-Asset reuse;
- current-turn media preview/resource delivery remains linked to a durable Asset record;
- cost/compute estimate, budget status, approval threshold, and hard-limit denial happen before job execution;
- public job submit/get/list/cancel lifecycle, ownership, cross-turn retrieval, and cancellation, plus operator-CLI-only synchronous execution with no public `wait` trigger;
- utility transforms preserve parent/child lineage instead of overwriting inputs;
- workflow registry and typed graph node/edge/port validation;
- graph invalidation, branch/parallel, partial-rerun, template instantiation, and underlying-effect composition;
- asset/provenance/revision lineage;
- SceneBoard/Director state remains engine-neutral;
- Game Design/Build Manifest and deploy-vs-publish lifecycle are distinct;
- effect/approval classification;
- disabled-by-default optional capability behavior.

### Execution-binding and MCP utility tests

Use fake/local mock bindings where possible to verify:

- semantic request validation plus binding-specific namespaced extension bounds;
- media-role validation including multi-reference and motion-control image/video role separation;
- explicit caller-selected binding, `execution_binding_required` on omission, and no MCP-side provider/model fallback;
- image generate/reference/edit/inpaint/upscale/remove-background/outpaint contracts, including one caller-selected conformance binding proving a 4K-class output;
- video generate/reference/image-to-video/extend/reframe/upscale/remove-background/motion-control contracts, including one caller-selected conformance binding proving >=15-second generation;
- long-video clip extraction preserves source timestamps/lineage and uses only contained/imported source Assets;
- timeout/cancellation/retry bounds;
- output bounds and current-turn media result delivery;
- durable result registration/history reuse;
- cost-estimate and budget-admission behavior;
- conformance binding bounds are reported and validated without promoting that binding/provider/model into a default MCP route;
- failure classification/redaction;
- endpoint/credential containment.

Do not require expensive GPU inference for deterministic repository tests.

### Blender protocol/security tests

Retain the original Plan 069 fake-bridge coverage:

- exact null-byte framing;
- loopback-only target;
- port/timeout validation;
- malformed/oversized responses;
- connection refusal/timeout;
- structured inspection injection resistance;
- import/export/checkpoint path containment;
- animation preview bounds;
- raw Python high-risk semantics;
- bounded activity/log content.

### SceneBoard / Director-spec tests

Verify with deterministic fixtures:

- upper-layer-produced Auto and Manual storyboard payloads validate into the same bounded Scene/Shot state;
- Element references and selected revisions resolve consistently across frames;
- frame-level revision preserves unrelated accepted frames;
- global scene direction and per-shot cinematography stay separate;
- MCP does not invent Director suggestions; caller-authored Director specs never auto-trigger generation merely by being stored;
- one Scene Manifest can execute through caller-selected fake generated-video bindings and Blender without changing source-of-truth state;
- continuity findings detect seeded cast/location/prop/light/screen-direction errors.

### Game/runtime tests

Verify through normal source/build/browser/runtime test ownership:

- Game Design/Build Manifest validation and stable asset paths;
- delayed/failed asset jobs do not corrupt source/project state;
- complete loop, win/lose/restart, missing assets, console/runtime failures, resize, and declared input methods;
- timing/physics behavior where applicable;
- source-preserving follow-up iteration;
- two-session room/state-sync behavior and cross-room isolation when online multiplayer exists;
- deployment maps to one accepted build revision and cannot silently become public publish.

### Upper-layer independence / MCP neutrality tests

Use at least two thin test clients with different orchestration assumptions to prove that MCP behavior is identical for equivalent semantic requests. Cover capability-unavailable behavior, missing required fields, execution-binding ambiguity/selection, job/graph chaining, Scene/Director manifest storage, QA evidence, revision scope, Game Manifest/build/playtest state, and deploy-vs-publish. Do not test agent prompt quality, routing, interviews, or model choice inside the MCP suite.

### Production fixture tests

Repository fixtures should test **contracts**, not subjective art quality. Examples:

- Character Pack can express canonical/interpreted views and Soul-Cast-like fictional fields;
- Element Library resolves Character/Location/Prop/Style selected revisions;
- Asset Manifest dependencies resolve;
- SceneBoard/Shot Manifest references existing Elements/assets;
- Creative Graph partial rerun invalidates only affected descendants;
- Game Design/Build Manifest binds stable runtime asset roles;
- QA finding lineage resolves to one revision/build;
- Blender character inspection can relate mesh/UV/rig/material/animation state;
- a contained user reference can reach generation/Blender/game asset use;
- export/handoff bundle contains required project metadata.

### Manual/operator quality acceptance

Actual scene/anime quality requires operator-enabled engines/Blender and visual judgment; actual game quality requires browser/runtime playtesting. Manual acceptance may evaluate aesthetics, identity/scene consistency, cinematography, motion, gameplay feel, and final output quality, but it must not substitute for deterministic security/contract/runtime tests.

## Failure handling and rollback

- **Bad generation direction:** return to accepted Style/Element revision; do not mutate it in place.
- **Identity/style/location drift:** reject candidate and retry/revise bounded fields; preserve parent lineage and unaffected Elements.
- **Bad SceneBoard/Director plan:** return to the accepted scene/board revision before high-cost motion; do not compensate with opaque prompt hacks.
- **Creative Graph node failure:** retain successful unaffected ancestors, mark descendants dirty, retry only within bounded policy, and never broaden node authority to make the graph pass.
- **Bad generated mesh:** discard bootstrap candidate and use another bootstrap/manual blockout; never force a poor mesh through rigging.
- **Topology/rig regression:** restore managed Blender checkpoint.
- **Game build regression:** return to the last accepted source/build revision, preserve unchanged assets/Elements, amend Game Manifest, then rerun complete playtest.
- **Game asset job failure:** keep stable manifest role/path and use bounded retry/explicit placeholder or manifest amendment; never loop forever.
- **Multiplayer failure:** disable/revert the selected multiplayer route or restore last accepted networking module; do not silently claim multiplayer from single-session success.
- **Deploy failure:** retain accepted local build and deployment metadata; do not publish or mutate source merely to force hosting success.
- **Execution-binding outage:** retain project/job state and fail with a capability/binding-specific diagnosis; MCP never silently switches to another provider/model binding. The upper layer may choose a replacement explicitly.
- **Capability unavailable:** report the missing operator capability/setup; continue independent planning/state work where possible.
- **Unsafe media/path:** reject before engine/Blender/game runtime sees it.
- **Template/resource regression:** roll back graph/template/guidance changes independently from execution bindings where contracts permit.
- **Provider/model quality regression:** do not change MCP routing because none exists; the upper layer may select a different compatible binding while Scene/Anime/Game MCP contracts remain unchanged.
- **Long-form/large-game failure:** fall back to the last passing track milestone; never broaden from a failing short scene or incomplete core loop to a full episode/large game.

## Alternatives rejected

### Rewrite Blender into a “Higgsfield clone”

Rejected because Blender is a DCC/execution backend, while the reusable platform value is capability/state/job/workflow/asset/QA infrastructure. Agent skills, routing, and model choice remain above the Masih Awam MCP boundary.

### Proxy or wrap Higgsfield MCP/CLI

Rejected because the user explicitly requires no Higgsfield account/runtime dependency, and it would keep auth, backend availability, billing, and model behavior outside Masih Awam ownership.

### Copy Higgsfield skill text wholesale

Rejected. Public skills are useful upper-layer reference material, but Plan 069 does not vendor them into an MCP-owned agent runtime. Learn their capability requirements and expose the necessary neutral MCP primitives/resources; let whichever upper layer owns agents/skills decide how to use them.

### Put every creative operation behind `terminal_exec`

Rejected because credentials, effects, long-running jobs, media results, capability discovery, and Blender host authority need typed first-class boundaries.

### Expose arbitrary executor-native workflow/code payloads through ordinary creative tools

Rejected because provider-native graphs/custom nodes/scripts can become a broad execution/supply-chain boundary. Prefer reviewed execution bindings and typed semantic/workflow contracts; executor-native escape hatches require their own privileged boundary.

### Add hundreds of Blender atomic tools

Rejected because a compact inspection/preview/I/O/recovery surface plus privileged `bpy` authoring and strong guidance gives more production flexibility with a smaller contract.

### Train a character model before proving reference-based consistency

Rejected as premature complexity. Character Pack + reference workflow is the baseline; training is added only when measured evals justify the cost/complexity.

### Target a full anime episode or large game first

Rejected because scale hides failures across Elements/identity, scene direction, 3D asset readiness, rigging, motion, continuity, gameplay loop, inputs, networking, and rendering. Short Scene S3, Anime A6, and Game G3/G5 benchmarks are the first integrated quality gates.

## Risks

### RISK-01 — Scope explosion

Creative production spans media generation, scenes, Blender/anime, browser games, graphs, and deployment. Mitigation: one shared C0–C3 kernel, then independently gated Scene S1–S4, Anime A1–A8, and Game G1–G6 ladders; marketing/site/analysis verticals remain post-core.

### RISK-02 — Provider/model quality changes independently from MCP contracts

Mitigation: semantic capabilities plus caller-selected opaque execution bindings keep provider/model IDs and quality-ranking policy out of MCP logic.

### RISK-03 — “Local” engine becomes arbitrary code execution

Mitigation: reviewed endpoints/workflows, custom-node trust boundary, disabled-by-default capability, no arbitrary model-supplied graphs in v1.

### RISK-04 — Character consistency remains weak

Mitigation: Character Pack, locked reference roles, visual QA, optional measured identity training, and Blender as the deterministic final asset authority.

### RISK-05 — 2D-to-3D bootstrap quality is insufficient

Mitigation: bootstrap is optional; manual/scripted Blender blockout and hybrid cleanup remain first-class.

### RISK-06 — Anime guidance becomes one-style dogma

Mitigation: Style Pack is project-specific; guides describe principles and strategies, not a universal shader/proportion preset.

### RISK-07 — Temporal QA is too shallow

Mitigation: explicitly distinguish sampled preview from full playback; expand multimedia result/playblast support only behind bounded contracts.

### RISK-08 — Optional creative guidance bloats connected-agent context

Mitigation: expose concise, progressive resources only when useful; MCP execution does not depend on loading creative manuals or a particular skill system.

### RISK-09 — Duplicate job systems

Mitigation: reuse the existing relay job/task manager unless a measured persistence/domain requirement proves a thin extension is necessary.

### RISK-10 — User references and generated interpretations become conflated

Mitigation: provenance schema marks authoritative user inputs separately from inferred/generated views and preserves original sources.

### RISK-11 — Canvas becomes a second unsafe execution framework

Mitigation: graph nodes map only to reviewed first-party capabilities/effects, templates are declarative and validated, partial execution preserves underlying approvals, and arbitrary shell/Python/custom-node payloads are never ordinary graph authority.

### RISK-12 — Scene direction becomes prompt-only and loses deterministic state

Mitigation: SceneBoard and Scene/Shot Manifests own Elements, hero frames, global look, cinematography, continuity, and revisions independently from generated prompt text or one video model.

### RISK-13 — Game Studio becomes a toy code generator

Mitigation: require Game Design/Build Manifest, stable generated assets, complete-loop browser playtest, declared input/runtime checks, source-preserving iteration, multiplayer verification when claimed, and deploy/publish separation before parity is claimed.

### RISK-14 — Online multiplayer or deployment expands authority too broadly

Mitigation: platform-owned bounded room/state-sync and deployment bindings own credentials/infrastructure; generated game source receives only the narrow runtime interface it needs and never generic operator credentials.

### RISK-15 — MCP parity media ingest becomes a host/network escape hatch

Mitigation: external-client upload uses expiring owner/project-bound first-party handoffs; URL import reuses SSRF/redirect/DNS/content-type/size policy; arbitrary local paths and engine-side downloads remain forbidden; all successful ingress resolves to a contained Asset ID before downstream use.

### RISK-16 — Execution-binding discovery leaks provider internals or becomes a hidden router

Mitigation: descriptors use opaque binding IDs and bounded capability/schema/license/estimate metadata, never credentials/endpoints; MCP never ranks/defaults/auto-selects bindings, missing binding fails with `execution_binding_required`, and incompatible requests fail precisely.

### RISK-17 — Budget checks stay prompt-only and fail to constrain batch execution

Mitigation: estimate/admission happens in the execution layer, operator hard maxima cannot be overridden by any caller/agent, and batch/graph execution accounts for aggregate budget before scheduling descendants.

## Final acceptance criteria

Plan 069 implementation is complete only when:

1. Masih Awam has no runtime dependency on Higgsfield services, auth, CLI, MCP, proprietary model IDs, or hosted state.
2. The first-party platform covers the relevant public Higgsfield capability classes—OAuth connection, image/video/3D generation and edit utilities, reusable character/Element references, safe upload/import/history reuse, durable job/result state, quota/cost visibility, and generic MCP-client control-plane operation—without copying Higgsfield's asynchronous execution model. Heavy work is synchronous foreground operator execution.
3. The parity claim remains bounded to **workflow architecture and production capability**; docs never claim identical proprietary model quality, private prompts, Higgsfield credit economics, marketplace implementation, or pixel-identical UI.
4. MCP remains agent-agnostic: no Plan 069 runtime path owns agent registries, agent spawning, skill auto-triggering, conversational interviews, creative routing, provider/model ranking, or fallback policy; optional resources/guidance carry no execution authority.
5. Creative Project, Element Library, Character, Style, World/Location, Asset, Scene/Shot, Game Design/Build, Creative Graph, QA/Playtest, and revision/provenance state are versioned, contained, and resumable.
6. Elements support at least Character, Location, Prop, Style, Media, 3D Asset, and Animation Clip selected revisions with explicit dependency impact.
7. Semantic capability discovery, opaque `execution_binding.list/get`, workflow `list/get`, asset/upload/history search, and budget/cost discovery are distinct first-party contracts rather than one ambiguous catalog.
8. MCP has no model/provider auto-selection policy: caller-selected `execution_binding_id` is required for pluggable executor-backed operations; omission fails with `execution_binding_required`; selected binding/workflow/compiler versions are recorded in job lineage and silent substitution is forbidden.
9. Creative jobs support public submit/get/list/cancel state, cross-turn retrieval, bounded retries/concurrency/results, owner/project isolation, current-turn media delivery, and durable Asset outputs; no public wait/poll execution trigger exists, and heavy execution is synchronous foreground operator work.
10. Cost/compute preflight plus enforceable job/batch/session/project hard limits can block expensive work before execution; paid-binding secrets/quota data remain binding-owned and redacted.
11. Conversation attachments, external-client device uploads, reviewed web-URL imports, prior generations, and promoted Elements can all become reusable stable Assets through reviewed ingest paths.
12. External-client upload handoff is OAuth/owner/project bound, expiring, bounded, and replay-safe; URL import obeys shared SSRF/redirect/DNS/content-type/size policy; arbitrary local host paths are never the normal MCP upload mechanism.
13. Generation/upload history is queryable by typed filters/source tags and prior media can be reused directly by Asset ID without download/re-upload round trips.
14. Finished media can be reviewed in the current MCP/client turn through a bounded media/resource result while the same output persists as a queryable Asset.
15. MCP-core image utility parity is implemented through normal semantic capability/execution-binding/workflow/job/lineage semantics: text/reference/mixed generation, editing, upscale, background removal, and outpaint; at least one explicitly selected conformance binding proves a 4K-class image output without becoming a platform default.
16. MCP-core video utility parity is implemented through normal semantic capability/execution-binding/workflow/job/lineage semantics: video generation/reference or image-to-video where active, reframe, upscale, background removal, motion control, and bounded clip extraction; at least one explicitly selected conformance binding proves a >=15-second video capability without becoming a platform default.
18. Every utility/edit transform creates a child revision/Asset and never mutates an accepted parent in place.
19. Creative Graph supports typed validation, branching, parallel-independent execution semantics, partial rerun, unaffected-output reuse, templates, node status/QA, and underlying approval/effect preservation; heavy graph execution/rerun is synchronous foreground operator work and is absent from public MCP actions.
20. A Canvas-style visual workspace edits/validates that same graph contract and can prepare/copy exact foreground execution commands without moving long-running execution, relay credentials, Blender host authority, provider secrets, or unrestricted executable node payloads into the browser.
21. SceneBoard contracts support connected frames, reusable Elements, frame-level revision, continuity constraints, hero-frame promotion, and `auto|manual` provenance; any actual Auto planning is performed by the upper layer and submitted through the same MCP contract.
22. Scene/Director state stores engine-neutral global look/lighting/palette plus per-shot framing/camera/lens/focal/aperture/movement/tempo; MCP validates/stores/admit caller-authored settings, while heavy shot execution is synchronous foreground operator work. MCP does not generate AI Director suggestions itself.
23. One fresh **S3 Scene Studio** benchmark produces a coherent 10–30 second cinematic scene with inspectable Scene/Shot/Element/job/QA state through generated-video, Blender, or a deliberate mixed backend.
24. Character identity supports a fictional-character path with structured visual/narrative fields independent of training; optional real-person identity-capable execution bindings require explicit intent and never replace Character Element authority.
25. Blender is a first-class optional capability with a loopback-only bridge and truthful host-authority semantics: exactly five bounded public MCP tools (`blender_session`, `blender_inspect`, `blender_python_api_docs`, `blender_screenshot`, `blender_animation_preview`), with arbitrary Python, final render, import/export, and checkpoint operations available only through the synchronous foreground operator CLI.
26. One anime character reaches production-ready-enough mesh/UV/material/hair/clothing state for the chosen benchmark, receives a reusable body rig plus initial facial controls, and passes representative deformation QA.
27. A 3–5 second anime character performance passes structural and temporal review.
28. One fresh **A6 Anime Studio** benchmark produces a 10–30 second anime sequence with SceneBoard/Director state, continuity, Blender-editable assets where selected, visual/temporal/render/export QA, and reusable Elements/assets.
29. Game Studio freezes Game Design/Build Manifest + STYLE FORMULA + Asset Manifest before broad asset/code work, supports parallel generated assets plus editable source, and preserves stable runtime paths/lineage.
30. One fresh **G3 Game Studio** benchmark passes a complete browser-game loop including start/core loop/win-lose/restart, declared inputs, missing-asset/runtime-console checks, responsive rendering, and timing/performance assertions appropriate to the game.
31. When multiplayer is claimed, the selected local/online path passes reviewed **G4** acceptance; online mode proves at least two-session room/state behavior and cross-room/owner isolation.
32. One fresh **G5 Game Studio** deployment maps a shareable playable URL to an accepted source/build revision while source remains editable and public marketplace/catalog publication has not happened implicitly.
33. Follow-up game requests preserve existing source/Elements/assets when unaffected, amend manifests deliberately, rerun affected graph/build stages, and repeat playtest before acceptance.
34. Structural/deterministic QA evidence distinguishes hard fail, soft finding, and not-inspected; subjective visual/temporal/creative judgments stay `not_inspected` unless supplied by the upper layer or an explicitly caller-selected evaluator binding; generation/build/deploy success is never treated as QA success by itself.
35. Surgical revision lineage can alter one accepted field/node/shot/asset/mechanic without overwriting unrelated locked state or provenance.
36. Generate/render/export/build, deploy/share, and public publish remain distinct lifecycle/effect boundaries.
37. A clean-room second-project falsification shows Scene, Anime, and Game contracts are not hard-coded to the first demo; at least one track reaches its repeatable-template milestone with materially different Elements/style/genre.
38. No caller/agent-supplied arbitrary provider endpoint, executor-native workflow/code payload, unrestricted Blender host/path, game networking credential, or secret-bearing generic terminal fallback bypasses reviewed capability boundaries.
39. Core MCP parity tests cover capability/execution-binding/workflow discovery, safe upload/import, history/reuse, cost/budget preflight, image/video utilities, job lifecycle, and media delivery in addition to Scene/Anime/Game production tests.
40. Website/App MCP parity is satisfied through existing generic editable-source/Git/file/build/test/deploy/publish capabilities plus creative Asset/Element imports, with all coding/design intelligence above MCP and deploy distinct from public publish. Extended non-core items—localization/subtitles/shorts, UGC/faceless/motion-design packaged workflows, identity edits, relight/weather/object edits, restore/stabilize/time-remap/color-match, and marketing verticals—have explicit shared-kernel owners and priorities; none requires a second state/job/asset platform.
41. Relevant focused tests and `pnpm guardrail:fast` / affected-stack full gates / `pnpm guardrail:full` pass before closure.
42. A generic external MCP client can complete the control-plane parity journey—discover capabilities/execution bindings/workflows, upload/import, estimate, submit generation/transform/graph state, browse history, reuse an Asset/Element, retrieve completed results, and hit a budget denial—without Higgsfield, arbitrary host paths, or an MCP-owned agent/model router. Any heavy execution step is deliberately completed by the human/operator through the exact synchronous foreground `ai-tools creative` command.
43. Operator-only actions such as executor/provider/model installation, Blender/add-on setup, relay restart, production deployment credentials, or external public publishing are reported explicitly and are not performed implicitly by Plan 069 MCP runtime.
44. Final Scene/Anime/Game/Blender acceptance uses canonical project roots beneath `$HOME/Documents/Projects`; Blender projects specifically run from `$HOME/Documents/Projects/Blender/<creative-project>/`. No production or final acceptance artifact rooted beneath the `ai-code` source checkout can satisfy closure.

## Historical consolidation note — non-authoritative

Earlier Plan 069 drafts, release branches, planning SHAs, and numbered tool-catalog snapshots are preserved only for audit/history. They are **not** alternate active implementations or versions. Git history is the source for detailed chronology.

The only active Plan 069 truth is this file plus the current `feat/plan-069-complete` implementation rebased conceptually on the reconciled `main` baseline above. The current architecture has one shared Scene/Anime/Game creative kernel, one runtime catalog composition path, one active Creative schema version (`1`), and one minimal Creative Graph executor that downstream phases must extend rather than replace.
