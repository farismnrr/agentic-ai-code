# Plan 069 — Creative Production Platform for Games, Scenes, and Anime

Status: **PLANNED — supersedes the Blender-only and anime-only Plan 069 scope before implementation; no production source implementation has started**

Created: 2026-09-11
Updated: 2026-09-13

## Goal

Build a first-party **Masih Awam Creative Production Platform** whose MCP layer provides model-agnostic and agent-agnostic creative capabilities, reusable Elements/assets, durable jobs, scene/anime/game production state, Blender/DCC execution, browser-game build/playtest primitives, QA evidence, and export/deploy boundaries **without depending on Higgsfield accounts, Higgsfield MCP, Higgsfield CLI, Higgsfield APIs, or Higgsfield-hosted generation**.

The strict interoperability benchmark is the **public Higgsfield MCP surface** documented in 2026: OAuth connection, media generation/edit utilities, reusable characters/Elements, audio operations, upload/import/history reuse, asynchronous jobs/results, quota/cost visibility, and MCP-compatible-client operation. Higgsfield's broader Canvas/Popcorn/Cinema/Games surfaces remain product references for the Scene/Anime/Game tracks, but Masih Awam deliberately does **not** copy Higgsfield's agent/model routing responsibility.

The architectural boundary is non-negotiable:

- **upper layer owns intelligence** — user interaction, interviews, prompt authoring, agent choice, model/provider choice, fallback policy, creative direction, and workflow/graph planning;
- **Masih Awam MCP owns capabilities and execution state** — validated semantic operations, project/Element/asset state, job lifecycle, graph execution of an already-specified DAG, Blender/DCC primitives, build/playtest/runtime primitives, deterministic/structural QA evidence, and export/deploy policy boundaries;
- **execution bindings are operator/runtime configuration, not product logic** — MCP may expose compatible opaque execution-binding descriptors so the upper layer can choose one, but MCP never decides that tool/capability A must use provider/model B;
- **no agent management** — Plan 069 does not add agent registries, agent spawning, role routing, skill auto-triggering, conversational interview logic, or model-selection policy to the MCP server.

Three production tracks are first-class from the architecture stage:

1. **Scene Studio** — script/brief -> Elements -> storyboard -> hero frames -> shot list -> camera/lighting/audio -> generated or Blender-backed cinematic scene.
2. **Anime Studio** — consistent fictional characters/worlds -> turnarounds -> optional 3D/Blender assets -> rig/animation -> multi-shot anime sequence.
3. **Game Studio** — game brief -> design/STYLE FORMULA -> asset manifest -> 2D/3D/audio assets -> playable browser build -> playtest -> optional multiplayer -> deploy, with public publication separate.

The first integrated release still advances incrementally: prove shared creative state and storyboard/scene quality first, then one short anime scene and one small verified browser game. A full anime episode, large game, or fully collaborative studio comes only after those smaller benchmarks pass.

## Success criteria

Plan 069 is successful when Masih Awam can eventually demonstrate all of the following through its own first-party contracts:

1. Any MCP-compatible upper layer can submit a fully specified creative operation or manifest and receive deterministic validation of required fields; conversational interviews and missing-information questions remain upper-layer behavior, not MCP behavior.
2. One **Creative Project** owns reusable Elements and manifests for characters, locations, props, style, audio, scenes/shots, game assets, generated outputs, QA, and provenance.
3. The stable MCP contract is **capability-centric, not agent/model-centric**: no public tool name, project schema, workflow identity, or guidance/resource requires a particular provider/model or agent implementation.
4. Runtime capability, workflow, and compatible execution-binding discovery can report validated semantic schemas before the upper layer commits to execution; discovery never ranks or auto-selects a model/provider/agent.
5. Generation/build jobs have first-party create/get/wait/cancel/result semantics, deterministic project ownership, bounded outputs, and retained source metadata.
6. A Canvas-style production graph can compose typed inputs, Elements, generation/edit nodes, storyboard/scene stages, Blender/DCC stages, game-build stages, QA, and delivery with partial reruns and reusable templates.
7. Reusable **Elements** provide stable project-scoped identities for Character, Location, Prop, Style, Audio/Voice, 3D Asset, Animation Clip, and approved media revisions; Elements can be reused across scenes, anime shots, and games.
8. Character identity supports both explicitly authorized real-person identity-capable execution bindings and **fictional-character creation** comparable to Soul Cast: structured appearance, outfit, archetype/personality/backstory, canonical views, later rig/voice bindings, and cross-scene consistency.
9. A Popcorn-like storyboard layer supports Auto and Manual planning modes, connected multi-frame boards, explicit reference roles, reusable Elements, and continuity of character/location/style/lighting/spatial logic before expensive video or animation work.
10. A Cinema Studio-like **Scene/Shot contract** can store and execute editable project-global style/lighting/palette rules plus per-shot camera/lens/focal/aperture/movement/framing/tempo controls; any AI Director that derives those settings from a script lives in the upper layer and submits the resulting manifest to MCP.
11. Blender remains a first-class persistent DCC backend for exact geometry, retopology, UVs, materials, hair/clothing, rigging, facial setup, animation, cameras, lighting, rendering, compositing, import/export, and checkpoints.
12. A Game Studio path can freeze a game design + STYLE FORMULA + asset manifest, generate 2D/3D/audio assets, build while independent jobs run, verify complete gameplay locally, support single-player and reviewed local/online multiplayer paths, deploy to a playable URL, and keep marketplace/public publication separate.
13. The system can inspect visual, temporal, structural, and gameplay outputs, reject failed results, and perform narrowly scoped revisions instead of blindly regenerating/rebuilding everything.
14. The first scene benchmark produces a coherent storyboard and **10–30 second cinematic scene** from shared Elements with inspectable shot/camera/audio/QA state.
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
- **Game Studio** methodology: game design, STYLE FORMULA, asset manifest, 2D/3D/audio generation, browser build, local playtest, multiplayer path, deploy, and explicit publish gate;
- single-player, local multiplayer, and reviewed online multiplayer browser-game contracts;
- visual, temporal, structural, and gameplay QA loops;
- audio/voice/music capability routing shared by scenes, anime, and games;
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
 image | video | audio | 3D | DCC | build | playtest | deploy
                      |
                      v
          caller-selected execution binding
         (opaque, operator-registered, validated)
                      |
      +---------------+----------------+----------------+
      |               |                |                |
 media executor   audio executor   3D executor     Blender / build runtime
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

An upper layer may request semantics such as `image.reference_generate`, `video.image_to_video`, `audio.voice`, `3d.image_to_mesh`, `dcc.execute`, `game.build`, `game.playtest`, or `game.deploy`, optionally with an explicit compatible `execution_binding_id`. Provider/model identifiers do not appear in the stable semantic contract, and MCP does not choose agents, models, providers, prompts, or creative direction.

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
| Generate | one connector across image/video/3D/audio, live schemas, jobs, reusable outputs | semantic capability surface + execution-binding discovery + durable jobs; upper layer chooses binding | **P0** |
| Canvas | node-based infinite production board; any model as node; branching/parallel compare; reusable workflow templates; partial execution | typed Creative Graph runtime first, then Nuxt visual graph editor; curated safe nodes; template save/reuse | **P0/P1** |
| Elements | reusable Characters, Locations, Props, saved outputs/reference media across shots/projects | project Element Library with Character/Location/Prop/Style/Audio/3D/Animation/Media types | **P0** |
| Soul ID | train/reuse a real-person identity across generation paths | optional authorized identity-capable execution bindings behind Character Elements, never the source of truth | **P1** |
| Soul Cast | construct fictional actors from structured character dimensions/backstory and reuse them across scenes | Fictional Character Builder -> Character Pack/Element -> canonical views/traits/outfit/personality/rig/voice bindings | **P0/P1** |
| Popcorn | Auto or Manual connected storyboards; multiple references; sequence-level character/light/atmosphere/spatial consistency; frame edits | SceneBoard with Auto/Manual shot planning, multi-frame board, Element references, continuity constraints, revision lineage | **P0/P1** |
| Cinema Studio | hero-frame-first filmmaking; script-to-shot AI Director; reusable Elements; global look controls; per-shot camera/lens/focal/aperture/moves; native audio; long/reference-heavy clips | MCP stores/validates Scene/Director state and executes caller-authored shots; AI directing stays in the upper layer | **P1 state/execution** |
| Video Explainer | style lock, script blocks, voice/video dependency ordering, deterministic assembly | generic sequence DAG and audio-first dependency patterns usable by scenes/anime | **P1** |
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
| Video upscaling | `video.upscale` | preserve source timing/audio where supported and report selected-binding limitations honestly |
| Image background removal | `image.remove_background` | transparent/derived asset with parent lineage |
| Video background removal | `video.remove_background` | alpha/matte or equivalent reviewed output contract; duration/resolution bounds |
| Image expand/outpaint | `image.outpaint` | aspect/canvas expansion without overwriting accepted parent revision |
| Video generation duration + reframe/expand | `video.generate` + `video.reframe` | binding descriptors expose duration/aspect bounds; parity acceptance includes at least one caller-selected video binding capable of a >=15-second clip and target reframe without making that binding a default |
| Motion-control generation | `video.motion_control` / typed motion reference | character/reference image and motion video have distinct roles; timing/source metadata retained |
| Reusable Soul characters | Character Elements + optional identity-capable execution binding | selected character revision can be referenced by name/ID across jobs without re-uploading source photos |
| Reusable reference Elements for characters, locations, props; several per prompt | Element Library | jobs accept multiple Element IDs/revisions and preserve dependency lineage |
| Voiceover / speech generation | `audio.speech` / `audio.voice` | language/performance metadata, contained output, timing metadata |
| Voice cloning | `audio.voice_clone` | explicit authorized reference/audio ingest, provenance, reusable voice Element, binding-specific artifact hidden behind Element contract |
| Voice change / conversion | `audio.voice_change` | source voice/audio -> derived audio with parent lineage and consent/usage policy |
| Video dubbing | `audio.video_dub` | source video + translation/voice plan -> synchronized derived video/audio assets |
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
| Generation is asynchronous and the connected agent polls/results arrive later | first-party creative job lifecycle | `submit/get/wait/list/cancel`, retained status, bounded failures, stable result assets, cross-turn retrieval; MCP does not require one long blocking agent turn |
| Separate workflow catalog from model catalog | `workflow.list/get` separate from `execution_binding.list/get` | higher-level chains have schemas/cost inputs/results and create normal jobs; execution bindings are discoverable but never selected by MCP policy |
| Full multi-step production through an MCP-connected agent | caller-specified Creative Graph + Scene/Game manifests + reusable Assets/Elements | the **upper layer/agent** owns skills, planning, interviews, creative decisions, and graph construction; MCP validates/executes the submitted graph/manifests without managing agents or auto-triggering skills |
| Website/App building through supported connected agents | existing generic workspace/Git/file/terminal/build/test/deploy capabilities + creative Assets/Elements | upper layer owns design/coding/model/agent choice; MCP supplies editable source/project operations and keeps build/deploy/public-publish authority distinct; no dedicated website agent is introduced |
| Browser game creation through MCP/Supercomputer surface | Game Manifest + generic workspace/build/playtest/deploy primitives + optional multiplayer binding | upper layer owns game design/code generation; MCP owns durable source/assets/build/playtest/deploy state and verification boundaries |
| Agent auto-selects a model when user does not specify one | **upper-layer responsibility; intentionally not implemented in MCP** | Higgsfield's own docs attribute this behavior to the connected agent. Masih Awam MCP exposes compatible execution bindings only; the caller supplies one for pluggable executor-backed operations. Omission returns `execution_binding_required`, never server-side selection/defaulting |
| User can force an exact model/provider in the connected experience | upper layer resolves that request to an explicit `execution_binding_id` | MCP honors the caller-selected compatible binding or rejects it precisely; it never silently substitutes or embeds provider/model names in the semantic tool contract |

### MCP parity rules

1. **Semantic capability, execution-binding, workflow, and asset/history catalogs are different concepts.** MCP does not own an agent/skill catalog. Do not collapse discovery into one ambiguous blob, and do not expose provider/model choice as a stable tool identity.
2. **Every transform is a first-class lineage operation.** Upscale, remove-background, outpaint, reframe, motion control, dubbing, and clip extraction produce child assets rather than overwriting accepted inputs.
3. **External-client upload is part of the product contract.** Conversation-local attachments alone do not satisfy MCP parity because a generic external MCP client may not be able to pass local file bytes directly.
4. **Media return and media persistence are separate guarantees.** A user should see/review the result in the current conversation and still be able to find/reuse it later by Asset ID.
5. **History is queryable state, not log scraping.** Generation/upload history uses project-owned records and typed filters rather than reading raw activity logs.
6. **Cost/budget behavior is explicit.** Local-first does not mean “free”; jobs may consume GPU time, provider quota, disk, or money. MCP reports/enforces measurable bounds for the caller-selected execution binding; the upper layer decides whether that cost is acceptable and whether to ask the user.
7. **Utility/edit operations belong to the same job/QA system as generation.** They must not become ad-hoc shell commands or direct engine calls.
8. **Client-specific omissions are not platform architecture.** Higgsfield's ChatGPT plugin currently omits some surfaces such as audio/website building; Masih Awam's core MCP contract should remain client-neutral and let each client expose the subset it can render/authorize.
9. **No agent identity is part of creative state.** Concurrent clients share owner/project state only through normal authorization and stable IDs; Plan 069 never stores an “active agent”, agent persona, model preference, or agent-specific routing state.
10. **Billing/account semantics are translated, not cloned.** Higgsfield subscription/credit rules are vendor-specific; Masih Awam parity is transparent estimate/quota/budget behavior for the caller-selected execution binding, not imitation of Higgsfield credits.

### Extended MCP parity backlog — advertised skills/post tools

The official MCP landing/blog material also demonstrates broader packaged workflows and post-production operations beyond the help-center core list. They are not blockers for the first Scene/Anime/Game milestones, but they must have an explicit home so future parity work does not invent a second platform:

| Advertised Higgsfield MCP/skill behavior | Masih Awam home | Priority |
| --- | --- | --- |
| batch/parallel campaign variants and presets | Creative Graph branches + templates + bounded batch jobs | P1/P2; core graph mechanism is P0 |
| UGC/product-review/SaaS creator flows | packaged Scene/Audio/Character/Subtitle templates | P3 vertical |
| faceless content (stickman, editorial motion graphics, whiteboard, watercolor, pixel-art, claymotion styles) | SceneBoard + Style Element + media generation + voice/subtitle + assembly templates | P2/P3; useful anime/motion-design overlap |
| localization | transcript/dialogue manifest + translation + `audio.video_dub` + subtitle track | P2 |
| subtitles/voiceover | Audio/Voice Pack + deterministic subtitle/timeline assembly | P1/P2 |
| Shorts Maker / repurpose | `video.clip_extract` + reframe + subtitle/template graph | P2 |
| motion design / animated infographics | Creative Graph + Scene Director + vector/text/layout/media nodes | P2 |
| face/character swap | reviewed identity-edit capability with explicit consent/provenance and lineage | P2, safety-gated |
| lighting/weather/background/object edits | structured image/video edit/inpaint specifications with scoped revision lineage | P2 |
| restore/stabilize/time-remap/auto-cut | post-production execution bindings under the same job/asset contract | P2 |
| color grading/reference color match | scene/style color specification + deterministic post-processing binding | P2 |
| Marketing Studio / ad multiplier | vertical skill/template over Elements + Scene/Audio/Graph; no second job/state system | P3 |
| Website Building / Apps | existing generic workspace/Git/file/build/test/deploy/publish lifecycle + creative Asset/Element imports; no managed coding agent | **MCP parity P1 contract reuse**, product templates/UI may remain later |

Any future extended utility must use the existing semantic capability/execution-binding/workflow discovery, safe ingest, Asset lineage/history, budget, jobs, QA, and graph authority. “Parity expansion” is not permission to add ad-hoc provider/model-specific public tools.

## What Higgsfield gets right — the operating model to reproduce

Higgsfield's strongest pattern is not any individual model. It separates **decision knowledge** from **execution capability**. Plan 069 preserves that separation even more strictly:

- Higgsfield-style skills/references, trigger rules, interviews, prompt strategy, model/provider choice, and creative decision trees are **upper-layer concerns** and may inspire clients/agents, but are not MCP server runtime;
- MCP exposes typed capability/workflow/execution-binding schemas instead of assuming static parameters forever;
- media has typed roles and validation before submission;
- generation/edit/build work creates durable jobs that can be waited on/retrieved later;
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
| Separate workflow discovery from implementation catalog | treats chains as first-class products | first-party workflow registry separate from execution-binding catalog | turnaround, rigging, lipsync, shot render are workflows; caller chooses compatible execution bindings |
| Media role validation | avoids malformed generation inputs | typed creative asset roles and selected-binding validation | `character_front`, `character_side`, `style_reference`, `motion_reference`, `voice_reference`, etc. |
| Auto-upload local path / reuse previous job output | lets outputs chain naturally | reviewed workspace materialization + stable asset IDs | concept art -> SceneBoard -> Blender/game asset -> shot/build |
| Canvas graph | keeps a whole multi-model workflow visible, branchable, reusable, and partially rerunnable | typed Creative Graph + saved templates + graph execution state | compare scene looks, branch anime variants, generate game assets in parallel without losing lineage |
| Elements | turns accepted characters/locations/props into reusable project assets instead of repeated prompt text | Element Library with typed references and selected revisions | same hero/location/prop reused by Scene Studio, anime shots, and Game Studio |
| Popcorn Auto/Manual storyboard | sequence consistency is solved before expensive video generation | SceneBoard Auto/Manual planning, connected frames, continuity rules, frame-level revision | direct a cinematic scene, anime board, or game cutscene with shared cast/location/style |
| Cinema Studio AI Director | script/idea becomes editable shot settings rather than an opaque monolithic generation | upper layer authors the Scene Manifest; MCP validates/stores/executes it | global style/lighting plus per-shot lens/focal/aperture/move/tempo remain engine-neutral state |
| Hero Frame First | locks composition/cast/location/look before motion makes changes expensive | approved hero frame/storyboard frame required before selected high-cost motion paths | cheaper scene/anime iteration and stronger continuity |
| Soul ID reusable identity | consistency survives many generations | Character Identity Pack | authorized real-person identity or recurring visual identity across scenes/media |
| Soul Cast fictional actor builder | invented characters need structured creation, not face training | Fictional Character Builder -> Character Element/Pack | anime/game actors with physique, outfit, traits, archetype/backstory, canonical views and later rig/voice |
| Brandkit reusable identity system | locks visual system before assets proliferate | Style Bible + World Bible | line language, shape language, color script, shader family, environments, typography/key-art rules |
| Domain prompt enhancer | encodes specialist production language | upper layer owns creative prompt/spec authoring; selected execution binding only performs deterministic syntax translation/validation | MCP does not invent creative prompts or choose a model |
| Product Photoshoot mode router | intent chooses workflow, not surface keywords | upper-layer routing over stable MCP primitives | MCP exposes the operations/state needed for portrait, full-body, action, environment, expression, or promo work but does not classify user intent |
| Thumbnail concept gate | concept is selected before costly render | upper-layer approval policy using MCP cost estimate + assets/manifests | MCP enforces explicit submit/budget/approval boundaries but does not choose the concept |
| Style preset resolve before explainer blocks | one style key stabilizes multi-shot output | Style Element/Pack stored by MCP; upper layer selects/locks it | all shots can reference one selected revision without MCP deciding style |
| Generate narration before dependent video blocks | freezes one modality before dependent generation | caller-specified dependency DAG executed by MCP | ordering comes from the submitted graph/manifest, not MCP creative planning |
| Explicit job `create/get/wait/list` | generation is durable work, not one RPC | first-party creative job lifecycle | long image/video/3D/audio jobs survive normal agent turns and can be referenced later |
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
| `higgsfield-video-explainer` | style lock + block planning + audio-first dependency + assembly | caller-specified scene DAG + deterministic assembly | **P1 MCP graph/state primitives; planning stays above** |
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
- audio/voice plan;
- Creative Graph/template references;
- generation/revision lineage references;
- source/provenance records;
- export/deploy/publish targets as separate lifecycle states;
- production, QA, playtest, and continuity findings.

Do not turn this into a database-first subsystem prematurely. Begin with workspace-contained structured files and promote to product persistence only when multi-session/product UX requires it.

### Element Library

Mirror the useful behavior of Cinema Studio Elements without tying identity to any one generation model. Every reusable creative object gets a stable Element ID, type, selected revision, references, provenance, and optional runtime bindings.

Initial Element types:

- `character` — Character Pack plus optional identity/rig/voice bindings;
- `location` — World/Location Pack plus 2D/3D scene bindings;
- `prop` — canonical appearance/scale/material/use references;
- `style` — Style Pack/reference set;
- `audio_voice` — recurring voice/performance identity;
- `media` — approved still/video/audio revision promoted for reuse;
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
- voice binding when created;
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
- type: character, prop, environment, clothing, hair, texture, effect, audio, shot intermediate, etc.;
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
- audio/dialogue/music context;
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
- dialogue/audio cue references;
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
- image/video/audio/3D generation;
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

### Audio / Voice Pack

For recurring characters and sequences:

- voice identity/reference plus optional caller-selected speech/voice execution binding;
- language/pronunciation notes;
- emotional/performance direction;
- dialogue timing artifacts;
- SFX/music references;
- licensing/provenance where relevant;
- lipsync/viseme timing output when available.

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

- list required character/prop/environment/audio assets;
- declare dependency relationships;
- define acceptable output/QA criteria per asset;
- choose which assets need 2D-only, 3D-only, or both.

### Gate D — SceneBoard / hero-frame / game-design lock

For scene/anime production, before high-cost motion or final animation:

- freeze scene intent and sequence order;
- produce/approve a connected storyboard or equivalent shot board;
- select representative hero frame(s) where the workflow benefits from Hero Frame First;
- freeze per-shot cast/location/prop Element revisions and camera intent;
- identify dialogue/timing dependencies;
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

- image/video/audio prompt text;
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
video.lipsync
video.clip_extract

audio.voice
audio.speech
audio.music
audio.sfx
audio.voice_clone
audio.voice_change
audio.video_dub

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

Mirror the useful Higgsfield job semantics as a first-party contract:

```text
submit -> queued/running -> completed|failed|cancelled
                 |                |
                 +---- get/wait --+
```

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

Jobs should be resumable/retrievable across normal agent turns. Exact persistence ownership is frozen only after auditing the post-v0.0.15 relay/product task model so we do not build a competing job manager unnecessarily.

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
- lipsync alignment where used;
- hair/clothing secondary motion;
- camera continuity;
- continuity between shots;
- flicker/style/identity drift in generated video;
- FX timing;
- audio sync.

The first version may use ordered frame/contact-sheet previews before video-native MCP result types exist. It must not pretend frame sampling equals full final playback QA.

## Creative Graph / Canvas execution model

The graph manifest above is not merely storage. It is a caller-authored orchestration contract that gives Masih Awam the useful execution behavior of Higgsfield Canvas while preserving stronger boundaries. MCP executes/validates graphs; it does not invent the graph or choose its agents/models.

### Required graph behavior

- construct a workflow without executing it;
- validate node schemas/asset-role compatibility before execution;
- connect one node output to one or many downstream consumers;
- branch variants from one accepted Element/output;
- run independent branches in parallel;
- compare/select results and promote one revision;
- rerun only the changed node and its invalidated descendants;
- retain accepted unaffected ancestors;
- save a graph as a reusable typed template;
- instantiate templates with new Characters/Locations/Props/Style without rewriting graph structure;
- show job/progress/QA state per node;
- keep generation/build/render/deploy/publish effects distinguishable;
- expose a compact MCP/tool surface even if the Nuxt UI eventually renders many node types.

### Initial safe node families

The first release should prefer a reviewed closed-world registry such as:

- `InputText`, `InputAsset`, `ElementRef`, `SelectRevision`;
- `GenerateImage`, `GenerateVideo`, `GenerateAudio`, `Generate3D`;
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
- execute selected node/subgraph/all valid dirty nodes;
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

MCP validates required fields/references, stores/version-controls the Director specification, estimates/enforces execution bounds, and executes requested shots through caller-selected bindings or Blender. It does **not** invent creative settings, choose an agent/model/provider, or infer an autonomy policy.

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
- assembled cinematic scene with audio;
- game cutscene package.

## Game Studio — first-class browser-game production

Game Studio is not a later “spinoff.” It is one of the three primary product tracks and reuses the same Elements, Style, Asset, Graph, 3D, audio, Blender, QA, and coding infrastructure.

### Game intake/output modes

- **design-only** — game profile + core loop + controls + level/world design + Asset Manifest;
- **assets-only** — sprites, UI, tileable textures, environments, rigged/animated 3D assets, music/SFX/voice;
- **build/iterate** — inspect existing source or create a new browser project, preserve unchanged architecture/assets, amend manifests, build, test;
- **deploy** — produce a shareable playable deployment after QA;
- **publish** — separate explicit public marketplace/catalog action when/if Masih Awam gains such a surface.

### Full game execution workflow

The **upper layer** authors game design and chooses execution/coding agents. MCP owns the durable manifests, assets, jobs, build/playtest/deploy primitives:

1. Receive/validate a Game Design/Build Manifest containing delivery context, core loop, win/lose/restart/progression, target devices, inputs, performance budget, language, and player-count mode.
2. Store/freeze the caller-approved Style Element / STYLE FORMULA and `design/assets` manifest before broad execution.
3. Receive the caller-selected multiplayer route: solo, local same-screen, or online room/state-sync.
4. Execute independent image/3D/audio jobs in parallel only when the submitted graph declares that independence.
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
4. no implicit Blender/add-on installation or service management;
5. raw Python is privileged host-user execution, not a sandbox;
6. read-only wrappers never accept arbitrary Python;
7. request/response bounds are relay-enforced;
8. workspace import/export/checkpoint/render paths are validated before Blender receives them;
9. URLs and arbitrary host paths are not asset-import shortcuts;
10. activity/logging does not persist giant scripts/scene dumps;
11. attachment ingress is explicit and contained;
12. no upper-layer coding system or Plan 069 implementation step restarts the live relay/systemd service implicitly.

### Retained Blender v1 tool surface

Keep the compact **11-tool** design:

1. `blender_status`
2. `blender_inspect`
3. `blender_python_api_docs`
4. `blender_execute_python`
5. `blender_screenshot`
6. `blender_animation_preview`
7. `blender_render`
8. `blender_asset_import`
9. `blender_asset_export`
10. `blender_checkpoint_create`
11. `blender_checkpoint_restore`

`blender_inspect` should retain scoped structured reads for at least `scene|object|mesh|uv|rig|animation|material|nodes|physics|asset|render|character`.

Ordinary authoring remains `bpy`-driven through the privileged execution tool rather than multiplying the catalog into hundreds of atomic wrappers.

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
- viseme/lipsync readiness;
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
  -> blocking / animation / dialogue timing
  -> hair/clothing/secondary motion
  -> camera / lighting / toon render
  -> temporal + visual QA
  -> composite / audio / export
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

MCP validates/executes the requested route but does not rank these options or choose one from quality heuristics.

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
- basic viseme/lipsync-ready shapes;
- checkpoint before destructive rig changes.

### Animation production

Begin with a small action, not a complete episode:

- idle/breathing;
- turn/look;
- walk/run or one action beat;
- expression transition;
- one short dialogue/performance beat when audio is available.

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
| S3 | 10–30 second cinematic scene | generated-video and/or Blender backend follows shot manifest, audio timing, continuity, visual/temporal QA |
| S4 | reusable scene template | materially different cast/location can reuse the graph/director workflow without source-code changes |

### Anime milestones

| Milestone | Deliverable | Must prove before advancing |
| --- | --- | --- |
| A1 | consistent anime still set | one fictional character remains recognizably consistent across controlled views/expressions |
| A2 | accepted turnaround + Style Pack | authoritative/interpreted references are distinguished; asset manifest is usable |
| A3 | Blender character asset | contained import, cleanup/retopo/UV/material inspection, multi-angle visual review |
| A4 | reusable rigged character | skeleton, weights, facial controls, representative deformation QA |
| A5 | 3–5 second animation | structural animation inspection + temporal preview + render review |
| A6 | 10–30 second anime scene | SceneBoard/shot manifest, continuity, camera/lighting/audio, final QA |
| A7 | 30–60 second multi-shot short | cross-shot continuity, reusable Elements/assets, render/composite pipeline |
| A8 | repeatable anime-production template | second project/character reuses platform without first-demo hard-coding |

### Game milestones

| Milestone | Deliverable | Must prove before advancing |
| --- | --- | --- |
| G1 | game design + STYLE FORMULA + Asset Manifest | core loop, win/lose/restart, inputs, budget, player mode, asset roles frozen before broad build |
| G2 | asset-complete local prototype | generated/procedural 2D/3D/audio assets resolve through stable manifest paths; full game loop runs locally |
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
| `blender_status` | read-only + privileged bridge identity | low/medium, no mutation |
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

## Repository baseline and dependency gate

Current `origin/main` baseline audited for this plan:

```text
e13bd38666ec4cb100ae713bd8272426db209d0e
```

Plan 069 is the next unused numeric plan on the audited `main` baseline as of 2026-09-13.

The `release/ai-tools-v0.0.15` work carries optional-capability composition that has not yet landed on `main`. The original Blender Plan 069 was kept only on that release branch. This rewritten Plan 069 is intentionally created from `origin/main` so future implementation has a canonical main-based planning artifact.

**Implementation gate:** before source work, re-audit current `main` and confirm the optional-capability framework from v0.0.15 (or its successor) is merged. Do not copy/cherry-pick an obsolete framework into a competing implementation.

## Expected implementation ownership

Exact new module names are frozen only after the Phase 1 architecture audit, but ownership should follow existing repository layers:

- `packages/rust-tools/src/core/config/` — operator capability config and bounds;
- `packages/rust-tools/src/application/` — first-party runtime/job/execution-binding/Blender application logic;
- `packages/rust-tools/src/interfaces/mcp/` — compact MCP tool schemas and capability metadata;
- `packages/rust-tools/src/application/resources.rs` or its post-v0.0.15 successor — bounded resources/capability guidance;
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
| PHASE-03 | Capability/workflow registry + creative jobs + graph contract | PHASE-01 | C2 contracts work without hard-coded model names; representative graphs validate |
| PHASE-04 | Initial reference-aware media generation | PHASE-02, PHASE-03 | Character/Style still consistency foundation passes A1/A2-quality gate |
| PHASE-05 | Identity/style/world/Element production system | PHASE-04 | reusable Characters/Locations/Props/Style Elements drive revisions and reuse |
| PHASE-06 | Blender first-class production engine | PHASE-02, PHASE-03 | retained 11-tool bridge/tool/security contract passes |
| PHASE-07 | Anime 3D character asset pipeline | PHASE-05, PHASE-06 | A3/A4 pass |
| PHASE-08 | Animation, facial, audio, and temporal QA | PHASE-07 | A5 passes |
| PHASE-09 | Scene Studio: SceneBoard + Director + shot orchestration | PHASE-04, PHASE-05; PHASE-06 optional per backend | S1/S2 pass and one S3 candidate can be produced/revised |
| PHASE-10 | Unified visual/temporal/structural QA + surgical revisions | PHASE-04, PHASE-08, PHASE-09 | failed scene/anime/media outputs are detected/revised with lineage |
| PHASE-11 | Sequence assembly/export + anime delivery | PHASE-08, PHASE-09, PHASE-10 | A6 plus contained reusable project delivery passes |
| PHASE-12 | Game Studio: design/assets/build/playtest | PHASE-02, PHASE-03, PHASE-05 | G1/G2/G3 pass for a small browser game |
| PHASE-13 | Game multiplayer/deploy + source-preserving iteration | PHASE-12 | selected G4 path passes where applicable and G5 deploy contract passes |
| PHASE-14 | Creative Graph executor + Canvas-style workspace + MCP parity/resource evals | PHASE-03 and proven scene/game workflows | C3 graph runtime/templates and high-value Higgsfield MCP operating parity pass |
| PHASE-15 | Triad acceptance and closeout | all required prior phases | fresh Scene S3, Anime A6, and Game G5 benchmarks pass; second-project falsification and repository gates pass |

# PHASE-01 — Reconcile baseline and freeze contracts

**Goal:** turn the benchmark into a repository-native architecture before implementation starts.

**Dependencies:** none.

### TASK-001 — Re-audit post-v0.0.15 runtime

**Outcome:** implementation starts from the actual merged optional-capability/task/resource architecture, not the release snapshot assumed by planning.

**Files:** read current `packages/rust-tools/src/`, `server/infrastructure/mcp/`, shared capability policy, and current contracts after updating from `main`.

**Steps:**

- [ ] Verify repository identity, clean task branch/worktree, and current `main` HEAD.
- [ ] Confirm which v0.0.15 optional-capability components landed.
- [ ] Re-audit current MCP protocol/tool/resource/task contract.
- [ ] Re-audit attachment/file ingress and image result support.
- [ ] Re-audit Blender Lab bridge/version/security docs.
- [ ] Re-audit the official Higgsfield MCP help pages/landing page operation list first; use public skills only as upper-layer/product reference, never as MCP authority.

**Validation:** architecture findings are recorded in this plan or durable knowledge without stale release-branch assumptions.

**Commit boundary:** `docs(plan): reconcile creative platform baseline` only if durable docs materially change.

### TASK-002 — Freeze the upper-layer / MCP responsibility boundary

**Outcome:** Plan 069 cannot accidentally turn the Masih Awam MCP server into an agent framework or model router.

**Files:** MCP catalog/resources, capability policy, creative contracts, and any existing prompt/subagent/skill integration only as boundary-audit inputs.

**Steps:**

- [ ] Document that agents, subagents, skill auto-triggering, interviews, creative prompt authoring, workflow planning, provider/model ranking, and fallback policy are outside Plan 069 MCP ownership.
- [ ] Define the semantic request envelope accepted from any upper layer.
- [ ] Define opaque `execution_binding_id` discovery/validation without provider/model-specific public tool names.
- [ ] Define the ambiguity error returned when multiple compatible bindings exist and the caller has not selected one.
- [ ] Define optional MCP resources/guidance as read-only reference material with no authority to auto-run or route an agent.
- [ ] Verify existing agent/subagent infrastructure is not extended merely to implement creative MCP parity.

**Validation:** architecture/contract tests can prove that two different upper-layer clients/agents can call the same creative MCP capability with different selected execution bindings and no MCP-owned agent/model-selection path is invoked.

**Commit boundary:** `docs(creative): freeze agnostic mcp boundary` or the smallest contract commit required during implementation.

### TASK-003 — Freeze creative project schemas

**Outcome:** versioned Creative Project, Element, character, style, world/location, asset, scene/shot, game, audio, graph/template, QA/playtest, and lineage contracts are specified before engine integration.

**Steps:**

- [ ] Define required vs optional fields.
- [ ] Define stable IDs and relative workspace references.
- [ ] Define authoritative vs interpreted/generated reference labels.
- [ ] Define revision lineage.
- [ ] Define forward-compatible schema versioning.
- [ ] Define bounds and secret-exclusion rules.

**Validation:** representative fixtures can express (a) one cinematic scene with Character/Location/Prop Elements and three shots, (b) one anime character/turnaround/rig path, and (c) one small browser-game design/build manifest, all with audio/provenance/revision lineage and no engine-specific IDs as source of truth.

**Commit boundary:** `feat(creative): add production manifest contracts`.

### TASK-004 — Freeze MCP-facing capability, execution-binding, workflow, budget, asset, and job contracts

**Outcome:** semantic capabilities, opaque execution-binding discovery, workflow discovery, budget/cost preflight, asset/history access, job lifecycle, effect policy, and result-media delivery are specified independently from agents/providers/models.

**Steps:**

- [ ] Freeze semantic capability vocabulary needed through C3/S3/A6/G5 plus MCP utility parity: upscale, background removal, outpaint/reframe, motion control, clip extraction, voice clone/change/dub.
- [ ] Freeze `capability.list/get` representation for durable upper-layer discovery.
- [ ] Freeze opaque `execution_binding.list/get` representation separately from semantic capabilities; provider/model names are optional implementation metadata, never stable contract identity.
- [ ] Freeze workflow `list/get` separately from execution-binding inventory.
- [ ] Freeze caller-selected-binding semantics and explicit ambiguity failure; MCP has no automatic model/provider selection or fallback policy.
- [ ] Freeze asset/upload/history `list/get/search` filters, source tags, stable IDs, and media-result representation.
- [ ] Freeze secure external-client upload request/complete and bounded URL-import contracts.
- [ ] Freeze cost/compute estimate plus global/project/session/job budget-status and hard-limit semantics.
- [ ] Freeze job submit/get/wait/list/cancel behavior and cross-turn retrieval.
- [ ] Freeze result and failure bounds, including current-turn preview/resource delivery plus durable Asset registration.
- [ ] Freeze external-cost/network vs local-compute effect classifications.
- [ ] Decide whether existing relay Tasks can own creative long-running jobs directly or need a thin creative domain layer over the same manager.

**Validation:** mock bindings can advertise different implementations for the same semantic capability without changing MCP contracts; the same fixture can discover compatible binding IDs, require the upper layer to select one when ambiguous, preflight cost for that binding, submit, list/retrieve the job, and reuse the returned Asset ID.

**Commit boundary:** `feat(creative): define capability and job contracts`.

**Phase exit criteria:**

- [ ] no implementation depends on a Higgsfield contract;
- [ ] the upper-layer/MCP boundary is explicit and no agent/model router was added;
- [ ] creative state schemas are frozen for initial milestones;
- [ ] execution-binding/job semantics reuse existing platform primitives where possible;
- [ ] Blender remains a separate privileged DCC capability under the master architecture.

# PHASE-02 — Creative Project, Element state, and safe media ingress

**Goal:** make references and generated outputs durable, contained production assets.

**Dependencies:** PHASE-01.

### TASK-005 — Implement contained creative project workspace layout

**Outcome:** each project has bounded manifests plus Element, asset/revision, SceneBoard, graph/template, and game-build state without escaping authorized workspaces.

**Steps:**

- [ ] Define contained path layout for project manifest, Elements, assets/revisions, scene/shot boards, creative graphs/templates, game design/build metadata, QA/playtest evidence, and exports.
- [ ] Ensure paths remain relative/canonical and cannot target protected credentials.
- [ ] Add atomic manifest updates.
- [ ] Preserve human-readable diffs.
- [ ] Bound manifest and metadata sizes.

**Validation:** traversal/symlink/protected-path tests fail closed; normal project fixture round-trips.

**Commit boundary:** `feat(creative): add contained project state`.

### TASK-006 — Implement MCP-safe media ingest: conversation attachment, device upload handoff, and URL import

**Outcome:** media from a Masih Awam conversation, an external MCP client's local device, a reviewed web URL, or a prior generated Asset can become a safe stable project Asset without arbitrary host-path assumptions.

**Steps:**

- [ ] Reuse an existing reviewed first-party conversation attachment handoff if present.
- [ ] Otherwise add the smallest owning-layer attachment materialization primitive.
- [ ] Add an external-client upload request/complete flow that returns an expiring OAuth-bound upload URL or equivalent reviewed first-party handoff rather than requesting arbitrary local filesystem paths from the agent.
- [ ] Bind upload requests to owner/project, media class, size limit, expiry, single-use/replay policy, and final Asset ID.
- [ ] Add bounded HTTP(S) URL import only through the existing SSRF/network policy: redirects, DNS/IP class, content type, size, timeout, filename and checksum/provenance are validated before persistence.
- [ ] Validate media type, size, destination ownership, filename/path, and source metadata for every ingest route.
- [ ] Never accept arbitrary host destination paths or let Blender/media engines fetch URLs directly as an ingest shortcut.
- [ ] Preserve source/provenance and source-mode (`conversation_upload`, `mcp_upload`, `url_import`, `generated_asset`, etc.).
- [ ] Keep model-visible attachment access distinct from Blender/engine-readable workspace files.

**Validation:** conversation-upload, external-upload-handoff, URL-import, and prior-Asset fixtures materialize to stable Asset IDs; expired/replayed upload tickets, private-network URL targets, redirect escapes, oversized/unsupported media, path injection, and owner/project mismatch fail closed.

**Commit boundary:** `feat(workspace): materialize creative attachments safely`.

### TASK-007 — Implement queryable asset/upload/history registry and revision lineage

**Outcome:** imported/generated/revised assets get stable project IDs, parent/child lineage, queryable history, and MCP-safe current-turn media delivery.

**Steps:**

- [ ] Register source type, source surface (`mcp|canvas|scene|anime|game|blender|manual/import` or equivalent), media role, project, Element binding, and selected revision.
- [ ] Record bounded output metadata/checksum/dimensions/duration where appropriate.
- [ ] Link generation/utility job and parent revision.
- [ ] Mark accepted vs candidate revisions.
- [ ] Implement bounded list/get/search filters for recent generations, uploads/imports, media type, job status, Element, project, source surface, and time range.
- [ ] Return reusable Asset IDs plus bounded preview/resource metadata so a result can be reviewed in the current client and reused later without download/re-upload.
- [ ] Keep raw credentials/prompts/unrestricted provider logs out of routine metadata.

**Validation:** fixtures can (a) trace user reference -> generated turnaround -> revised selected image -> Blender import, (b) list recent generated/uploaded assets with stable filters, and (c) reuse one previous Asset directly as a later job reference.

**Commit boundary:** `feat(creative): track asset revision lineage`.

**Phase exit criteria:**

- [ ] production media can be safely materialized;
- [ ] every asset has bounded provenance/lineage;
- [ ] no Blender/generator path assumes transient chat paths are filesystem paths.

# PHASE-03 — Capability/workflow registry, creative jobs, and graph contract

**Goal:** reproduce Higgsfield's live discovery/job strengths without vendor coupling.

**Dependencies:** PHASE-01.

### TASK-008 — Add semantic capability, execution-binding, and workflow discovery

**Outcome:** any client/agent can discover durable semantic capabilities, compatible opaque execution bindings, and workflows with validated schemas without MCP ranking/selecting a provider/model.

**Steps:**

- [ ] Add operator-disabled-by-default creative capability group.
- [ ] Expose active semantic capabilities independently from execution implementations.
- [ ] Expose bounded `execution_binding.list/get` descriptors: opaque binding ID, capabilities, reference roles, validated extension schema/bounds, duration/resolution/aspect constraints, availability/health, known license notes, and estimator support.
- [ ] Expose workflow IDs and `workflow.list/get` separately from execution bindings.
- [ ] Never rank/default/auto-select a binding. For pluggable executor-backed operations, omission returns `execution_binding_required` with compatible binding IDs regardless of how many bindings are currently registered.
- [ ] Validate an explicit caller-selected binding and return precise incompatibility/unavailable diagnostics instead of silent substitution.
- [ ] Keep endpoint/credential/raw engine implementation metadata hidden; provider/model display metadata, if exposed at all, is informational and non-contractual.
- [ ] Return activation/setup hint when disabled.

**Validation:** semantic capabilities remain stable while underlying provider/model implementations change; the same request works through two caller-selected mock bindings, missing selection always fails with `execution_binding_required`, and incompatible binding selection fails with a bounded reason.

**Commit boundary:** `feat(creative): expose capability discovery`.

### TASK-009 — Add first-party creative job lifecycle, cost preflight, and enforceable budgets

**Outcome:** generation/transform work can be estimated, approved when required, started, polled, waited, listed, cancelled, and retrieved with stable project ownership and bounded spend/compute authority.

**Steps:**

- [ ] Reuse existing job/task manager lifecycle and cancellation where possible.
- [ ] Add `cost.estimate`/compute-estimate semantics before submit when the selected execution binding/workflow can provide meaningful data.
- [ ] Add `budget.status` for configured provider quota/credits, local compute class/quota, disk/output bounds, and project/session/job hard limits where measurable.
- [ ] Enforce operator/user-configured thresholds before expensive jobs/batches; approval cannot override an operator hard maximum.
- [ ] Add domain metadata only where creative workflows need it.
- [ ] Support submit/get/wait/list/cancel and cross-turn retrieval.
- [ ] Bound concurrent jobs/batches and retry count.
- [ ] Persist/recover only if current platform task semantics cannot satisfy cross-turn retrieval safely.
- [ ] Keep failure/cost/provider diagnostics redacted/classified.

**Validation:** fake execution bindings cover queued/running/completed/failed/cancelled, list/retrieve after client reconnect, estimate-before-submit, approval threshold, hard-budget denial, timeout, output bounds, and owner isolation.

**Commit boundary:** `feat(creative): add generation job lifecycle`.

### TASK-010 — Add curated workflow registry and Creative Graph contract

**Outcome:** multi-step workflows are discoverable independently from execution bindings, and a typed graph can represent caller-specified composition before a visual Canvas/executor is added.

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
- `lipsync_preview`
- `image_upscale`
- `video_upscale`
- `image_remove_background`
- `video_remove_background`
- `image_outpaint`
- `video_reframe`
- `video_motion_control`
- `video_clip_extract`
- `voice_clone`
- `voice_change`
- `video_dub`
- `shot_render_preview`
- `game_asset_batch`
- `game_build_playtest`

Steps:

- [ ] Define workflow schemas independently from execution-binding/provider/model catalogs.
- [ ] Define typed Creative Graph node/edge/port/revision contract around workflows/jobs/Elements.
- [ ] Define dirty-descendant and partial-rerun semantics.
- [ ] Define template input/output contract.
- [ ] Validate that graph representation cannot encode arbitrary code/endpoints/credentials as ordinary nodes.

**Validation:** workflows advertise required inputs/output roles and reject unsupported parameters before execution; representative scene/anime/game graphs validate without containing engine-specific secrets or arbitrary executable payloads.

**Commit boundary:** `feat(creative): add workflow and graph contracts`.

**Phase exit criteria:**

- [ ] semantic capability, execution-binding, workflow, asset/history, and budget discovery are distinct and queryable;
- [ ] explicit caller-selected binding works and omitted execution binding fails without MCP-side ranking/defaulting/auto-selection;
- [ ] core MCP utility workflows are represented: upscale, background removal, outpaint/reframe, motion control, clip extraction, voice clone/change/dub;
- [ ] job lifecycle is bounded, listable/retrievable across normal turns, and owner/project scoped;
- [ ] cost/compute preflight and hard budget thresholds can block a batch before execution;
- [ ] graph/workflow contracts can express scene/anime/game dependencies;
- [ ] arbitrary upper layers can make their own routing decisions without hard-coded provider/model IDs in MCP contracts.

# PHASE-04 — Initial reference-aware media generation

**Goal:** prove the reference/identity/style generation substrate through the anime A1/A2 still benchmark before solving full 3D production, while keeping the public MCP contract independent from the conformance execution binding used for the test.

**Dependencies:** PHASE-02, PHASE-03.

### TASK-011 — Implement the media execution-binding contract and core image utility surface

**Outcome:** at least one conformance binding can perform reference-aware image generation/editing plus image-side MCP parity utilities, while the public capability contract remains independent of that binding/provider/model.

**Steps:**

- [ ] Freeze one provider/model-neutral execution-binding interface for media operations before choosing any conformance implementation.
- [ ] Confirm each registered binding's operator setup, endpoint policy, executable/plugin trust model, result format, cancellation, license metadata, and credentials remain implementation details.
- [ ] Implement/route supported image capabilities through the same job/lineage contract: generate, reference-generate, edit/inpaint, upscale, remove-background, and outpaint; pose/depth controls remain capability-gated per binding.
- [ ] Make execution-binding discovery expose supported utility/reference roles and validated extensions without changing semantic tool names.
- [ ] Keep arbitrary executor-native graphs/custom code disabled in ordinary capability calls.
- [ ] Add operator registration/configuration docs without declaring a product-wide default provider/model.

**Validation:** one fake binding plus at least one operator-selected conformance binding pass the same contract tests for reference generation and child-asset transforms; replacing the binding requires no MCP schema/tool-name changes and no Higgsfield dependency.

**Commit boundary:** `feat(creative): add media execution binding`.

### TASK-012 — Implement structured prompt/spec compiler

**Outcome:** an upper-layer-authored, engine-neutral creative request compiles deterministically into the caller-selected binding's validated input shape with traceable versioning.

**Steps:**

- [ ] Define provider/model-neutral semantic request spec.
- [ ] Define reference role/order.
- [ ] Accept creative prompt/content from the upper layer; do not author or enhance it inside MCP.
- [ ] Translate only reviewed semantic fields plus namespaced binding extensions.
- [ ] Record binding/compiler version in job lineage.
- [ ] Support controlled changed-fields for revision requests.

**Validation:** the same semantic request serializes without provider/model names and compiles deterministically through two different mock bindings selected by the caller.

**Commit boundary:** `feat(creative): compile structured generation specs`.

### TASK-013 — Prove Character/Style Element execution primitives with an upper-layer-driven still workflow

**Outcome:** an external client/agent can use MCP state/jobs/assets to create and curate a stable front/side/back/three-quarter/expression reference set without MCP owning interviews, style decisions, prompt strategy, or model choice.

**Steps:**

- [ ] Accept a caller-authored Style/Character specification and selected execution binding.
- [ ] Store the caller-approved Style Element revision.
- [ ] Execute requested canonical/reference views and additional views/expressions with typed reference roles.
- [ ] Label generated hidden views as interpreted.
- [ ] Return media plus deterministic metadata/evidence for upper-layer visual review.
- [ ] Let the caller promote accepted revisions into the Character Element/Pack through explicit state mutation.

**Validation:** an external upper-layer acceptance client can drive A1/A2 using only agnostic MCP contracts; MCP preserves identity/style/reference lineage and never chooses the creative direction/model or claims subjective QA without caller/evaluator evidence.

**Commit boundary:** `feat(anime): build character reference workflow`.

**Phase exit criteria:**

- [ ] one character can be reproduced across controlled stills;
- [ ] style and character state are reusable;
- [ ] visual QA can reject/promote revisions;
- [ ] accepted turnaround is ready for 3D production.

# PHASE-05 — Identity, Style, World, and Element production system

**Goal:** turn the first still workflow into reusable Character/Location/Prop/Style Elements rather than demo prompts, so the same state can feed Scene Studio, Anime Studio, and Game Studio.

**Dependencies:** PHASE-04.

### TASK-014 — Add dependency-aware Style Pack revisions

**Outcome:** style changes identify affected downstream assets rather than silently drifting the project.

**Validation:** palette/line/material rule change marks dependent candidate assets/shots for review while preserving accepted unrelated state.

**Commit boundary:** `feat(creative): track style dependencies`.

### TASK-015 — Support optional identity-capable execution bindings without replacing Character Pack authority

**Outcome:** embeddings/LoRA/fine-tunes or future identity mechanisms can improve consistency while remaining implementation details of caller-selected bindings.

**Steps:**

- [ ] Measure reference-only baseline first.
- [ ] Expose training/identity preparation only when an upper layer explicitly requests a compatible binding/capability; MCP never decides that training is needed.
- [ ] Store training artifact/license/version lineage.
- [ ] Keep Character Pack references and design constraints authoritative.

**Validation:** deleting/changing one binding-specific identity artifact does not erase the character's project identity contract or force changes to MCP tool/schema identity.

**Commit boundary:** `feat(anime): support identity-capable execution bindings`.

### TASK-016 — Add World/Location Pack

**Outcome:** recurring locations/environment language can be reused across key art and shots.

**Validation:** two shots can reference the same location pack while varying time/camera without losing core location identity.

**Commit boundary:** `feat(anime): add reusable world packs`.

**Phase exit criteria:**

- [ ] style/character/world state survives multiple jobs;
- [ ] binding-specific identity preparation is optional, traceable, and replaceable;
- [ ] downstream impact of style changes is explicit.

# PHASE-06 — Blender first-class production engine

**Goal:** implement the retained original Plan 069 Blender capability on the merged optional-capability framework.

**Dependencies:** PHASE-02, PHASE-03.

### TASK-017 — Freeze Blender bridge/config/tool schemas

**Outcome:** operator config, loopback framing, 11 tool schemas, inspection scopes, docs source, bounds, effects, and approval policy are frozen against current Blender Lab integration.

**Validation:** immutable prior contracts remain untouched unless the post-v0.0.15 versioning policy explicitly requires a successor snapshot.

**Commit boundary:** `feat(blender): freeze relay capability contract`.

### TASK-018 — Implement bounded loopback Blender bridge

**Outcome:** Rust relay connects only to the reviewed local add-on protocol with null-delimited JSON framing and bounded errors.

**Validation:** fake TCP bridge tests cover framing, malformed/oversized replies, timeout, refusal, cancellation where supported, and impossible arbitrary host targeting.

**Commit boundary:** `feat(blender): add loopback bridge`.

### TASK-019 — Implement structured Blender read/knowledge/preview tools

**Outcome:** status, inspection scopes, version-aligned API/manual lookup, screenshots, animation previews, and bounded render previews work without arbitrary Python input.

**Validation:** fake-bridge integration tests cover every scope/result transformation and preview bound.

**Commit boundary:** `feat(blender): add structured production reads`.

### TASK-020 — Implement contained Blender asset/recovery tools

**Outcome:** import/export/checkpoint create/restore respect workspace/protected-path authority and reviewed formats.

**Validation:** path escape, URL, unsupported format, arbitrary destination, and checkpoint forgery tests fail closed.

**Commit boundary:** `feat(blender): add contained asset and checkpoint tools`.

### TASK-021 — Implement privileged Blender Python authoring

**Outcome:** `blender_execute_python` enables coherent live modeling/rigging/animation edits with high-risk/manual semantics.

**Validation:** plan mode/approval/activity behavior is correct; code/results are bounded/redacted; no claim of sandboxing inside Blender.

**Commit boundary:** `feat(blender): add privileged authoring bridge`.

### TASK-022 — Compose optional capability/resources/docs

**Outcome:** Blender appears only when enabled; capability resources, activation hints, routing guidance, docs, and tool catalog agree.

**Validation:** disabled/enabled catalog tests plus repository Rust guardrail.

**Commit boundary:** `feat(blender): compose optional capability`.

**Phase exit criteria:**

- [ ] all 11 Blender tools are implemented and tested;
- [ ] no generic stdio/nested Python MCP server is required;
- [ ] Blender filesystem/network/host authority is represented honestly;
- [ ] safe reference materialization integrates with Blender import.

# PHASE-07 — Anime 3D character production

**Goal:** reach A3/A4 using the accepted Character/Style Elements/Packs and Blender engine.

**Dependencies:** PHASE-05, PHASE-06.

### TASK-023 — Implement 2D-reference -> Blender character bootstrap workflow

**Outcome:** accepted turnaround references become a contained Blender production setup.

**Steps:**

- [ ] Materialize/import front/side/back references.
- [ ] Accept the bootstrap route chosen by the upper layer: caller-selected image-to-3D execution binding, Blender/manual blockout, or hybrid.
- [ ] Validate that the selected route/binding is active and compatible; MCP does not rank bootstrap methods or choose one from quality heuristics.
- [ ] Preserve source/reference alignment metadata.
- [ ] Inspect multi-angle silhouette before detail work.

**Validation:** fixture can reproduce the same bootstrap from project state and contained assets.

**Commit boundary:** `feat(anime): bootstrap character in blender`.

### TASK-024 — Production mesh/UV/material pass

**Outcome:** character becomes a reusable deformation-ready asset rather than a generated mesh artifact.

**Steps:**

- [ ] Inspect topology/normals/transforms.
- [ ] Retopologize or repair as required.
- [ ] Establish UV readiness.
- [ ] Build toon/stylized material system from Style Pack.
- [ ] Add eyes/hair/clothing/accessories with chosen strategy.
- [ ] Capture multi-angle visual QA.
- [ ] Checkpoint accepted asset state.

**Validation:** mesh/UV/material/character inspection + rendered views meet fixture gates.

**Commit boundary:** `feat(anime): produce reusable character asset`.

### TASK-025 — Rig, weights, facial controls

**Outcome:** character supports representative body and facial animation.

**Steps:**

- [ ] Build/reuse reviewed humanoid armature strategy.
- [ ] Skin and inspect weights.
- [ ] Add constraints/IK/FK only where workflow benefits.
- [ ] Test representative deformation poses.
- [ ] Add eye/jaw/expression controls.
- [ ] Add initial viseme/lipsync-ready shapes.
- [ ] Checkpoint and export contained reusable asset.

**Validation:** A4 deformation/facial fixture passes structural + visual review.

**Commit boundary:** `feat(anime): rig reusable character`.

**Phase exit criteria:**

- [ ] one project character is reusable in a fresh scene;
- [ ] topology/UV/material/rig/facial state is inspectable;
- [ ] representative deformation is visually accepted;
- [ ] asset can be exported and re-imported within contained workspace authority.

# PHASE-08 — Animation, facial, audio, and temporal QA

**Goal:** reach A5 with real motion, not only rig existence.

**Dependencies:** PHASE-07.

### TASK-026 — Add audio/voice execution-binding contract with MCP parity for speech, cloning, conversion, and dubbing

**Outcome:** caller-selected reviewed audio execution bindings can create speech/voice/music/SFX and, where activated, perform authorized voice cloning, voice change, and video dubbing through one contained audio lineage model.

**Steps:**

- [ ] Freeze `audio.voice|speech|music|sfx|voice_clone|voice_change|video_dub` contracts and media/reference roles.
- [ ] Preserve voice/source/license/consent provenance and distinguish generated fictional voices from authorized reference-voice derivatives.
- [ ] Bind reusable accepted voice state to an `audio_voice` Element rather than exposing binding/provider-specific training IDs as the project identity.
- [ ] Keep credentials/training artifacts isolated if a non-local provider is supported.
- [ ] Produce contained audio/video child assets plus timing/language/voice metadata needed for animation and scene workflows.
- [ ] Keep dubbing/voice conversion independently revisable from body animation or source video.

**Validation:** fixtures cover bounded speech plus mocked/activated clone, voice-change, and dub contracts; unauthorized/missing reference inputs fail closed and project/parent lineage is preserved.

**Commit boundary:** `feat(audio): add anime dialogue capability`.

### TASK-027 — Add pose/action execution primitives

**Outcome:** any upper layer can author a short action using inspected rig state, API docs/resources, checkpoints, and preview while MCP exposes the editable Blender execution/state primitives.

**Validation:** action/F-curve/keyframe/NLA state is structurally visible and representative frames show intended motion.

**Commit boundary:** `feat(anime): add character action workflow`.

### TASK-028 — Add facial/lipsync workflow

**Outcome:** dialogue timing can drive bounded facial/viseme animation with editable Blender state.

**Validation:** lipsync is reviewed temporally and can be corrected without rebuilding the body animation.

**Commit boundary:** `feat(anime): add facial performance workflow`.

### TASK-029 — Add secondary motion review

**Outcome:** hair/clothing secondary motion can be authored or deliberately omitted with explicit reasoning.

**Validation:** physics/constraint state is inspectable and clipping/explosion failures are caught in sampled preview.

**Commit boundary:** `feat(anime): add secondary motion workflow`.

**Phase exit criteria:**

- [ ] a 3–5 second character performance exists;
- [ ] structural animation state and temporal preview agree;
- [ ] audio/facial/body motion remain separately editable;
- [ ] failures can roll back to a checkpoint.

# PHASE-09 — Scene Studio state/execution: SceneBoard, Director specification, and shots

**Goal:** reach S1/S2 and produce an S3 candidate while keeping storyboard/directing intelligence above MCP: MCP stores/validates caller-authored SceneBoard/Director state and executes requested shots through caller-selected bindings.

**Dependencies:** PHASE-04, PHASE-05. PHASE-06/08 are required only for Blender-backed animated shots; generated-video scenes use caller-selected compatible execution bindings.

### TASK-030 — Implement SceneBoard + Scene/Shot Manifest contracts

**Outcome:** MCP can store, validate, version, and revise a connected board and bounded scene/shot plan supplied by any upper layer before render-heavy work starts.

**Steps:**

- [ ] Support `planning_mode=auto|manual` as provenance metadata, but **do not implement AI auto-planning inside MCP**: in `auto`, the upper layer supplies the generated board; in `manual`, the upper layer/user supplies explicit per-frame/shot direction.
- [ ] Validate the same board/shot schema regardless of which upper layer produced it.
- [ ] Bind Character/Location/Prop/Style Elements with selected revisions.
- [ ] Track global scene look/lighting/atmosphere/spatial constraints across frames.
- [ ] Allow frame-level revision while preserving unrelated accepted frames.
- [ ] Promote selected frames to hero-frame candidates.
- [ ] Freeze shot IDs/durations/assets/continuity/camera intent.
- [ ] Validate required upstream Elements/assets exist.
- [ ] Keep approval/effect boundaries explicit; conversational autonomy policy remains upper-layer behavior.

**Validation:** two different upper-layer fixtures (one labeled auto, one manual) can submit/revise the same SceneBoard contract without hidden chat context; changing one frame preserves other accepted frame identities/lineage.

**Commit boundary:** `feat(scene): add sceneboard and shot planning`.

### TASK-031 — Implement Director-spec validation + generated-video utility surface + shot execution DAG

**Outcome:** caller-authored global scene settings and per-shot cinematography can be validated/executed through selected bindings, while generated-video executors expose reusable transform/job semantics expected from the MCP parity matrix.

**Steps:**

- [ ] Add global scene settings: genre/look, Style Element, lighting, color palette, atmosphere, era/time where relevant.
- [ ] Add per-shot settings: shot size/framing, camera profile, lens/focal/aperture/DoF intent, move, movement speed/stabilization, tempo/edit intent.
- [ ] Store a caller-selected Hero Frame First route when requested; MCP does not decide when it is creatively beneficial.
- [ ] Execute a shot through the caller-selected generated-video binding, Blender path, or explicit mixed graph without changing the engine-neutral manifest.
- [ ] Route active video capabilities through semantic capability/execution-binding/job/asset contracts: text/reference/image-to-video generation, extend, reframe, upscale, remove-background, and motion-control when supported.
- [ ] Validate distinct media roles for motion-control inputs (character/reference image vs motion-reference video) and preserve timestamp/duration lineage.
- [ ] Expose video utility workflows outside Scene Studio too; a user should be able to reframe/upscale/remove-background an existing Asset without manufacturing a Scene Manifest.
- [ ] MCP accepts already-authored Director settings/prompts from the upper layer; it neither invents Director suggestions nor silently triggers generation.
- [ ] One failed shot/utility transform invalidates/retries only its affected descendants.

**Validation:** the same Scene Manifest can execute through two caller-selected mock video bindings and a fake/real Blender path with consistent Element/camera intent; separate fixtures prove reframe, upscale, background removal, and motion-control child-asset lineage/job semantics without MCP choosing the backend.

**Commit boundary:** `feat(scene): add director and shot orchestration`.

### TASK-032 — Implement scene continuity and carry-forward anchors

**Outcome:** cross-shot character, costume, prop, location, screen direction, lighting, action, camera, and state continuity is reviewed before final output.

**Steps:**

- [ ] Compare Element revisions and scene-enter/exit state across shots.
- [ ] Track location geography/light direction/important landmarks.
- [ ] Track action/prop handoff and screen direction.
- [ ] Track selected hero/reference anchor used to carry continuity into later generated sequences.
- [ ] Emit bounded findings that can be resolved with scoped revision.

**Validation:** seeded scene continuity fixtures produce detectable findings, and a selected prior frame can be registered as the next-sequence anchor without overwriting original provenance.

**Commit boundary:** `feat(scene): validate shot continuity`.

**Phase exit criteria:**

- [ ] S1 SceneBoard state supports upper-layer Auto/Manual experiences through one agnostic contract;
- [ ] S2 global/per-shot Director controls are represented independently from engines and authored outside MCP;
- [ ] one 10–30 second S3 candidate has bounded shot state;
- [ ] reusable Character/Location/Prop/Style Elements survive shot changes;
- [ ] generated-video and/or Blender preview can be inspected and revised shot by shot;
- [ ] active video utilities (reframe/upscale/remove-background/motion-control where supported) are independently callable through normal capability/execution-binding/workflow/job/asset contracts.

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

- [ ] generation success is no longer conflated with QA success;
- [ ] revision lineage is first-class;
- [ ] accepted work can be refined without full regeneration by default.

# PHASE-11 — Scene/anime composite, export, and reusable delivery

**Goal:** turn accepted Scene Studio / Anime Studio shots into contained deliverables and reusable Elements/assets without hidden deployment/publication.

**Dependencies:** PHASE-09, PHASE-10.

### TASK-037 — Implement deterministic sequence assembly and Personal-Clipper-style extraction

**Outcome:** accepted shot renders/audio can be assembled deterministically, and an existing long-form video Asset can be analyzed/segmented into bounded reusable clips without bypassing normal ingest, job, and lineage rules.

**Steps:**

- [ ] Assemble accepted shots with explicit ordering, frame rate, resolution, transitions, and audio sync.
- [ ] Implement `video.clip_extract` over a contained/imported source Asset with bounded duration/input size.
- [ ] Preserve source start/end timestamps, transcript/segment metadata where available, parent Asset ID, and selected clip revisions.
- [ ] Allow explicit user-selected time ranges and an optional reviewed clip-selection workflow; do not silently download arbitrary third-party media outside the safe URL-import path.
- [ ] Register extracted clips as normal child Assets that can feed SceneBoard, Canvas, Game, or export workflows.

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

- [ ] final media and reusable project assets are contained;
- [ ] long-form source video can produce reusable timestamp-lineaged clip Assets through the bounded clip-extraction workflow;
- [ ] no publish/deploy action is hidden in export;
- [ ] a later session can continue from project state.

# PHASE-12 — Game Studio: design, assets, build, and playtest

**Goal:** reach G1/G2/G3 with one small but complete browser game using the same Elements, Style, Asset, job, and QA foundations as Scene/Anime Studio.

**Dependencies:** PHASE-02, PHASE-03, PHASE-05. PHASE-06 is optional for games that need Blender-authored 3D/rigging.

### TASK-040 — Implement Game Design/Build Manifest validation and state

**Outcome:** MCP can validate/store/version an upper-layer-authored Game Design/Build Manifest and STYLE FORMULA before broad asset execution or build work.

**Steps:**

- [ ] Accept the design-only/assets-only/build/deploy intent resolved by the upper layer.
- [ ] Validate required genre, perspective, target devices, core loop, verbs, win/lose/restart/progression, player count, controls, camera, language, physics/timing, and performance/asset budgets.
- [ ] Freeze solo/local/online multiplayer route.
- [ ] Bind shared Character/Location/Prop/Style Elements where applicable.
- [ ] Write stable `design/assets` roles/paths before generated asset batches.
- [ ] Define placeholder policy and missing-asset behavior.
- [ ] Keep public publish outside the build/deploy workflow.

**Validation:** two different upper-layer clients can submit the same valid G1 manifest and drive identical MCP build-state behavior; required gameplay/style/input fields fail deterministically when absent, without MCP asking conversational questions.

**Commit boundary:** `feat(game): add game production manifest`.

### TASK-041 — Implement parallel game asset generation and source integration

**Outcome:** image/3D/audio generation jobs can run independently while editable game source is built against stable manifest identities.

**Steps:**

- [ ] Add game-specific media roles for sprites, UI, tileables, sky/environment, textures, 3D models, animation clips, music, SFX, and voice where needed.
- [ ] Execute independent asset jobs concurrently only when the caller-specified graph/manifest declares them independent; MCP does not creatively schedule undeclared work.
- [ ] Build source against manifest-stable placeholders/paths rather than ephemeral job filenames.
- [ ] Promote accepted results into Elements/assets and replace placeholders deterministically.
- [ ] Validate skeleton/action compatibility for animated 3D assets; route to Blender/retargeting when necessary.
- [ ] Preserve source and asset lineage separately.

**Validation:** a fixture can replace delayed generated assets after code exists without manual filename edits or source-wide rebuilds unrelated to those assets.

**Commit boundary:** `feat(game): integrate generated production assets`.

### TASK-042 — Implement reviewed browser-game source/runtime templates

**Outcome:** any upper-layer coding system can create/edit a normal workspace project for a small 2D or 3D browser game using MCP workspace/build/runtime primitives without depending on an opaque hosted builder or MCP-managed coding agent.

**Steps:**

- [ ] Audit current web/coding stack before freezing templates/runtime family.
- [ ] Keep simple 2D and 3D paths separate when their runtime needs materially differ.
- [ ] Keep game source human-editable and versionable.
- [ ] Bind asset/runtime paths through the manifest.
- [ ] Add reviewed resize/input/timing patterns rather than unconstrained generated infrastructure.
- [ ] Avoid hard-coding one framework into the Creative Project contract.

**Validation:** one 2D-or-3D fixture builds and serves locally through normal repository/runtime tooling with no Higgsfield dependency.

**Commit boundary:** `feat(game): add browser game runtime templates`.

### TASK-043 — Implement Game QA / local playtest contract

**Outcome:** a build cannot be called complete from compile success alone.

**Steps:**

- [ ] Serve the built game over HTTP/runtime preview.
- [ ] Verify start -> core loop -> win/lose -> restart.
- [ ] Detect missing assets and uncaught console/runtime errors.
- [ ] Verify responsive/canvas render and declared keyboard/mouse/touch/gamepad inputs.
- [ ] Verify fixed-step/timing/physics assumptions where relevant.
- [ ] Measure bounded performance indicators chosen during implementation.
- [ ] Emit hard-fail/soft/not-inspected Game QA findings with source/build revision.

**Validation:** seeded broken-loop, missing-asset, input, console-error, and timing fixtures fail correctly; G3 benchmark requires a passing complete loop.

**Commit boundary:** `feat(game): add browser playtest qa`.

**Phase exit criteria:**

- [ ] G1 game design/style/assets are frozen before broad work;
- [ ] G2 source and generated assets integrate through stable identities;
- [ ] G3 small browser game passes complete-loop local playtest;
- [ ] source remains editable and resumable by a fresh agent session.

# PHASE-13 — Game multiplayer, deploy, and source-preserving iteration

**Goal:** add the high-value Supercomputer/Games behavior after the single-player/local core is proven: reviewed multiplayer when selected, shareable deploy, and continued source iteration without conflating deployment with publication.

**Dependencies:** PHASE-12.

### TASK-044 — Implement multiplayer route and room/state-sync module

**Outcome:** Game Studio can distinguish solo, same-screen/local multiplayer, and online multiplayer without giving generated game code unrestricted infrastructure authority.

**Steps:**

- [ ] Keep local multiplayer inside the normal client/game runtime when possible.
- [ ] Define one platform-owned bounded online room/state-sync interface when online mode is enabled.
- [ ] Define room identity, join/leave, authoritative/shared state, update bounds, reconnect/failure behavior, and cleanup.
- [ ] Prevent generated client code from receiving deployment/database/provider credentials directly.
- [ ] Add two-session acceptance fixtures for online claims.

**Validation:** two independent sessions can join one disposable room and observe the reviewed synchronized state; cross-room/owner leakage fails closed.

**Commit boundary:** `feat(game): add bounded multiplayer runtime`.

### TASK-045 — Implement game deploy as a distinct lifecycle

**Outcome:** a QA-passing build can become a shareable playable deployment while public listing/publish remains a separate action.

**Steps:**

- [ ] Reuse current application/deployment ownership where practical instead of inventing a second hosting plane.
- [ ] Freeze deploy input to one accepted build revision.
- [ ] Return deployment identity/URL and bounded status.
- [ ] Preserve source/project link to the deployed build.
- [ ] Never publish to a public gallery/marketplace implicitly.
- [ ] Treat production deployment authority according to existing product approval/policy.

**Validation:** deploy fixture targets only a reviewed deployment surface; `game.publish`/public listing is absent or requires a distinct explicit action.

**Commit boundary:** `feat(game): separate deploy from publication`.

### TASK-046 — Implement source-preserving game iteration

**Outcome:** caller-requested follow-up changes can amend an existing game rather than silently regenerating it from scratch.

**Steps:**

- [ ] Inspect existing design/source/assets before mutation.
- [ ] Amend the Game Manifest when requested behavior changes.
- [ ] Reuse unchanged Elements/assets/source modules.
- [ ] Invalidate only affected graph/jobs/build outputs.
- [ ] Re-run complete playtest after gameplay-impacting changes.
- [ ] Keep deployment revision history explicit.

**Validation:** a follow-up feature request changes one mechanic/asset while unrelated accepted game behavior and assets remain intact.

**Commit boundary:** `feat(game): support iterative game production`.

**Phase exit criteria:**

- [ ] G4 passes whenever multiplayer is selected for the benchmark;
- [ ] G5 shareable deployment is tied to one accepted source/build revision;
- [ ] deploy and publish remain different authority boundaries;
- [ ] follow-up iteration preserves source/project continuity.

# PHASE-14 — Creative Graph executor, Canvas workspace, and MCP parity evals

**Goal:** complete C3 and high-value Higgsfield MCP parity by turning caller-specified graph contracts into a safe reusable executor/workspace over proven scene/anime/game primitives, without adding an agent/skill/model router.

**Dependencies:** PHASE-03 plus working scene/game workflows from PHASE-09/12. Blender/anime graph nodes depend on their owning phases.

### TASK-047 — Implement typed Creative Graph executor

**Outcome:** validated graphs can execute reviewed nodes with branch/parallel/partial-rerun semantics while preserving underlying tool authority.

**Steps:**

- [ ] Resolve graph dependencies deterministically.
- [ ] Start independent eligible nodes concurrently within existing job/admission bounds.
- [ ] Record node run status, output asset IDs, QA, cost/compute metadata where available.
- [ ] Mark invalidated descendants after input/Element revision changes.
- [ ] Reuse accepted unaffected ancestors.
- [ ] Support selected-node/subgraph/all-dirty execution.
- [ ] Map every graph node to an existing reviewed capability/tool/effect rather than granting graph-wide authority.

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

- [ ] Render pannable graph workspace with typed ports/nodes.
- [ ] Show Element/media thumbnails, selected revisions, node status/errors, and QA summaries.
- [ ] Support branch/compare/select and template save/load.
- [ ] Support run selected node/subgraph/all dirty nodes through server/relay ownership.
- [ ] Keep secret credentials and Blender host authority server/relay-side.
- [ ] Defer realtime multi-user collaboration while keeping project/node identities collaboration-ready.

**Validation:** browser graph edits round-trip to the same validated graph contract used headlessly; UI cannot fabricate unsupported node authority.

**Commit boundary:** `feat(creative): add visual production canvas`.

### TASK-050 — Add MCP contract/resource parity eval scenarios

**Outcome:** Generate/Assets/Elements/Scene/Anime/Game/Graph MCP contracts are testable without evaluating any particular agent's routing quality.

Eval categories include missing/invalid fields, unavailable capability, ambiguous execution binding, explicit binding selection, upload-handoff vs URL-import vs prior-Asset reuse, generation/upload history lookup, estimate-before-submit and budget denial, utility transforms (upscale/background removal/outpaint/reframe/motion control/clipper/dub), Element preservation, graph execution, caller-authored SceneBoard Auto/Manual provenance, Director-spec validation, Game Manifest validation, deterministic QA/playtest hard failures, revision lineage, deploy-vs-publish, and no authority escalation through templates.

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

- [ ] C3 graph execution/partial rerun/template behavior passes;
- [ ] visual Canvas edits the same underlying graph contract;
- [ ] core MCP capability/state/workflow parity is evaluated deterministically;
- [ ] packaged workflows reuse, rather than bypass, Elements/Graph/QA.

# PHASE-15 — Scene + Anime + Game acceptance and closeout

**Goal:** prove Masih Awam is one coherent creative production system across all three requested targets, not three unrelated demos or a collection of tools.

**Dependencies:** all required prior phases.

### TASK-052 — Run fresh S3 Scene Studio benchmark

**Outcome:** one 10–30 second cinematic scene is produced from a fresh Creative Project through Elements -> SceneBoard -> Director -> hero frame -> shot execution -> continuity/QA -> assembly/export.

**Validation:** scene can use generated-video, Blender, or a deliberate mix; every shot retains engine-neutral Scene/Shot state and reusable Element references; no hidden chat-state teleportation exists.

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
8. body/facial/audio timing;
9. temporal review and scoped corrections;
10. camera/lighting/toon render/compositor;
11. final assembly/audio sync;
12. contained export and reusable Elements/assets;
13. project handoff bundle.

**Validation:** every stage leaves inspectable Element/asset/job/scene/QA evidence; no hidden Higgsfield dependency.

### TASK-054 — Run fresh G5 Game Studio benchmark

**Outcome:** one small but complete browser game is produced from a fresh project through Game Design -> STYLE FORMULA -> Asset Manifest -> parallel asset/build work -> local playtest -> optional multiplayer route -> accepted deploy.

**Validation:** full loop/win-lose/restart and declared input/runtime checks pass; generated assets resolve through manifest identities; source remains editable; if multiplayer is claimed, two-session acceptance passes; deployed URL maps to the accepted build; no public marketplace publish happened implicitly.

### TASK-055 — Run clean-room second-project falsification

**Outcome:** prove the architecture is not hard-coded to the first character, scene genre, or game genre.

**Validation:** without source-code special cases, materially different fixtures can reach at least S1/S2, A1/A2, and G1/G2 using the same Elements/Graph/MCP contracts from different upper-layer clients; at least one track reaches its repeatable-template milestone.

### TASK-056 — Run security and failure matrix

**Outcome:** re-test path escapes, protected credentials, engine endpoint policy, arbitrary workflow/code/template injection, graph authority composition, job ownership/cancellation/output bounds, Blender host authority, malicious media metadata, game networking isolation, multiplayer cross-room isolation, build/deploy/publish separation, and production deployment authority.

**Validation:** relevant focused Rust/Nuxt/browser/runtime tests plus security review pass.

### TASK-057 — Run generic external MCP parity acceptance

**Outcome:** prove the creative platform is usable as a real MCP creative connector from a generic authenticated external MCP client, not only from Masih Awam's own UI or a local coding shell.

Acceptance journey:

1. connect through the normal Masih Awam OAuth-protected MCP endpoint with no creative-provider key exposed to the client;
2. inspect semantic capabilities, compatible opaque execution bindings, and workflows separately;
3. query budget/cost status and preflight one small generation;
4. create an external-client upload handoff (or use a safely imported URL) and resolve it to an Asset ID;
5. submit/wait/retrieve one bounded generation and receive both an in-conversation media/resource result and durable Asset ID;
6. run at least one core utility transform on that Asset (for example upscale, background removal, outpaint/reframe as appropriate) and verify child lineage;
7. list/search recent generations/uploads and reuse a previous Asset or Element directly in a second job;
8. run one caller-specified multi-step graph/workflow through the same connector surface without any MCP-owned skill/agent runtime;
9. register/discover two compatible mock execution bindings, verify the caller-selected binding is honored, verify an incompatible binding fails precisely, and verify omission returns `execution_binding_required` rather than triggering MCP default/auto-selection;
10. verify a configured hard budget/approval threshold prevents an over-budget request before execution;
11. use a generated Asset in one editable website/app project driven by a thin upper-layer test client, then exercise existing source/build/test/deploy primitives and verify public publish remains a separate action with no MCP-managed coding agent.

**Validation:** the flow requires no Higgsfield account/CLI, no local shell command, no arbitrary host path, and no manual download/re-upload between jobs. Client-specific rendering differences are allowed, but the MCP contracts and stable IDs remain identical.

### TASK-058 — Repository closure

**Steps:**

- [ ] Run focused subsystem tests while iterating.
- [ ] Run `pnpm guardrail:fast` before checkpoint commits.
- [ ] Run affected full Rust/Nuxt gates as required by changed ownership.
- [ ] Run browser/runtime game acceptance where game behavior changed.
- [ ] Run `pnpm guardrail:full` before closure.
- [ ] Run dependency/security audits when dependency changes justify them.
- [ ] Update operator docs, architecture/security docs, optional upper-layer resources/guidance, canonical memory, and this plan's status/checklists truthfully.
- [ ] Review `.agents/knowledge/self-improvement.md`.
- [ ] Deliver through short-lived branch -> PR -> reviewed merge to `main`; do not bypass hooks or self-merge without authorization.
- [ ] Keep relay restart, GPU/model installation, Blender/add-on setup, production deployment, and public publishing as explicit external/operator actions where policy requires them.

**Phase exit criteria:**

- [ ] S3 Scene benchmark passes;
- [ ] A6 Anime benchmark passes;
- [ ] G5 Game benchmark passes, including G4 evidence when multiplayer is claimed;
- [ ] C3 graph/template/Canvas parity is proven;
- [ ] generic external MCP parity acceptance passes without CLI/shell/manual file teleportation;
- [ ] second-project falsification shows generality;
- [ ] security/failure matrix passes;
- [ ] docs and runtime contracts agree;
- [ ] no Higgsfield runtime dependency exists;
- [ ] repository closure gates pass.

## Test strategy

Use repository-native test locations only. Do not add `verify-069` or other plan-numbered scripts.

### Contract tests

Verify:

- Creative Project/Element/asset/scene/shot/game/audio/graph/QA schema versioning and bounds;
- Element selected-revision and dependency-impact semantics;
- semantic capability discovery independent from provider/model/binding implementation names;
- execution-binding list/get schema, mandatory caller selection for pluggable executor-backed operations, `execution_binding_required` on omission, and precise incompatible/unavailable failure;
- workflow registry remains separate from execution-binding inventory and exposes validated inputs/estimator metadata where available;
- external MCP upload request/complete expiry, owner/project binding, single-use/replay handling, media/size bounds, and stable Asset result;
- bounded URL import uses shared SSRF/network policy and rejects redirect/private-network/content-type/size violations;
- asset/upload/generation history list/get/search filters, source tags, stable IDs, and previous-Asset reuse;
- current-turn media preview/resource delivery remains linked to a durable Asset record;
- cost/compute estimate, budget status, approval threshold, and hard-limit denial happen before job execution;
- job submit/get/wait/list/cancel lifecycle, ownership, cross-turn retrieval, and cancellation;
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
- audio speech/music/SFX plus authorized voice-clone/voice-change/video-dub contracts;
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

Rejected because scale hides failures across Elements/identity, scene direction, 3D asset readiness, rigging, motion, continuity, gameplay loop, inputs, networking, rendering, and audio. Short Scene S3, Anime A6, and Game G3/G5 benchmarks are the first integrated quality gates.

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
2. The first-party MCP surface covers the public Higgsfield MCP capability classes 1:1 where they are relevant—OAuth connection, media generation/edit utilities, reusable character/Element references, audio operations, safe upload/import/history reuse, async job/result delivery, quota/cost visibility, and generic MCP-client operation—while broader Canvas/Scene/Game contracts provide the requested Scene/Anime/Game production substrate.
3. The parity claim remains bounded to **workflow architecture and production capability**; docs never claim identical proprietary model quality, private prompts, Higgsfield credit economics, marketplace implementation, or pixel-identical UI.
4. MCP remains agent-agnostic: no Plan 069 runtime path owns agent registries, agent spawning, skill auto-triggering, conversational interviews, creative routing, provider/model ranking, or fallback policy; optional resources/guidance carry no execution authority.
5. Creative Project, Element Library, Character, Style, World/Location, Asset, Scene/Shot, Game Design/Build, Audio, Creative Graph, QA/Playtest, and revision/provenance state are versioned, contained, and resumable.
6. Elements support at least Character, Location, Prop, Style, Media, Audio/Voice, 3D Asset, and Animation Clip selected revisions with explicit dependency impact.
7. Semantic capability discovery, opaque `execution_binding.list/get`, workflow `list/get`, asset/upload/history search, and budget/cost discovery are distinct first-party contracts rather than one ambiguous catalog.
8. MCP has no model/provider auto-selection policy: caller-selected `execution_binding_id` is required for pluggable executor-backed operations; omission fails with `execution_binding_required`; selected binding/workflow/compiler versions are recorded in job lineage and silent substitution is forbidden.
9. Creative jobs support submit/get/wait/list/cancel, cross-turn retrieval, bounded retries/concurrency/results, owner/project isolation, current-turn media delivery, and durable Asset outputs.
10. Cost/compute preflight plus enforceable job/batch/session/project hard limits can block expensive work before execution; paid-binding secrets/quota data remain binding-owned and redacted.
11. Conversation attachments, external-client device uploads, reviewed web-URL imports, prior generations, and promoted Elements can all become reusable stable Assets through reviewed ingest paths.
12. External-client upload handoff is OAuth/owner/project bound, expiring, bounded, and replay-safe; URL import obeys shared SSRF/redirect/DNS/content-type/size policy; arbitrary local host paths are never the normal MCP upload mechanism.
13. Generation/upload history is queryable by typed filters/source tags and prior media can be reused directly by Asset ID without download/re-upload round trips.
14. Finished media can be reviewed in the current MCP/client turn through a bounded media/resource result while the same output persists as a queryable Asset.
15. MCP-core image utility parity is implemented through normal semantic capability/execution-binding/workflow/job/lineage semantics: text/reference/mixed generation, editing, upscale, background removal, and outpaint; at least one explicitly selected conformance binding proves a 4K-class image output without becoming a platform default.
16. MCP-core video utility parity is implemented through normal semantic capability/execution-binding/workflow/job/lineage semantics: video generation/reference or image-to-video where active, reframe, upscale, background removal, motion control, and bounded clip extraction; at least one explicitly selected conformance binding proves a >=15-second video capability without becoming a platform default.
17. MCP-core audio parity is implemented through normal Asset/Element lineage: speech/voice, authorized voice cloning, voice change, and video dubbing; music/SFX remain supported semantic capabilities when a caller-selected binding provides them.
18. Every utility/edit transform creates a child revision/Asset and never mutates an accepted parent in place.
19. Creative Graph execution supports typed validation, branching, parallel independent nodes, partial rerun, unaffected-output reuse, templates, node status/QA, and underlying approval/effect preservation.
20. A Canvas-style visual workspace edits/runs that same graph contract without moving relay credentials, Blender host authority, provider secrets, or unrestricted executable node payloads into the browser.
21. SceneBoard contracts support connected frames, reusable Elements, frame-level revision, continuity constraints, hero-frame promotion, and `auto|manual` provenance; any actual Auto planning is performed by the upper layer and submitted through the same MCP contract.
22. Scene/Director state stores engine-neutral global look/lighting/palette plus per-shot framing/camera/lens/focal/aperture/movement/tempo; MCP validates/executes caller-authored settings and does not generate AI Director suggestions itself.
23. One fresh **S3 Scene Studio** benchmark produces a coherent 10–30 second cinematic scene with inspectable Scene/Shot/Element/job/QA state through generated-video, Blender, or a deliberate mixed backend.
24. Character identity supports a fictional-character path with structured visual/narrative fields independent of training; optional real-person identity-capable execution bindings require explicit intent and never replace Character Element authority.
25. Blender is implemented as the retained 11-tool first-class optional capability with loopback-only bridge and truthful host-authority semantics.
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
39. Core MCP parity tests cover capability/execution-binding/workflow discovery, safe upload/import, history/reuse, cost/budget preflight, image/video/audio utilities, job lifecycle, and media delivery in addition to Scene/Anime/Game production tests.
40. Website/App MCP parity is satisfied through existing generic editable-source/Git/file/build/test/deploy/publish capabilities plus creative Asset/Element imports, with all coding/design intelligence above MCP and deploy distinct from public publish. Extended non-core items—localization/subtitles/shorts, UGC/faceless/motion-design packaged workflows, identity edits, relight/weather/object edits, restore/stabilize/time-remap/color-match, and marketing verticals—have explicit shared-kernel owners and priorities; none requires a second state/job/asset platform.
41. Relevant focused tests and `pnpm guardrail:fast` / affected-stack full gates / `pnpm guardrail:full` pass before closure.
42. A generic external MCP client can complete the parity acceptance journey—discover capabilities/execution bindings/workflows, upload/import, estimate, generate, receive media, transform, browse history, reuse an Asset/Element, run a caller-specified multi-step graph, and hit a budget denial—without Higgsfield, CLI, local shell, arbitrary host paths, or an MCP-owned agent/model router.
43. Operator-only actions such as executor/provider/model installation, Blender/add-on setup, relay restart, production deployment credentials, or external public publishing are reported explicitly and are not performed implicitly by Plan 069 MCP runtime.

## Current execution state

As of 2026-09-13:

- `origin/main` is `e13bd38666ec4cb100ae713bd8272426db209d0e` at the planning baseline; implementation must re-audit current `main` again before source work;
- Plan 069 did not exist on that `origin/main` baseline; 069 was the next unused numeric plan there;
- the earlier Blender-only Plan 069 lived on the local `release/ai-tools-v0.0.15` branch and had no Blender source implementation started;
- the retained Blender design still owns its loopback bridge, security, 11-tool surface, 3D/anime workflow guidance, attachment ingress, QA, asset I/O, and checkpoint decisions;
- the first rewrite broadened the target to an anime-first Creative Production Platform, but this second review found that framing too narrow for the user's actual target;
- the top-level product target is now explicitly **one Creative Production Platform with first-class Scene Studio, Anime Studio, and Game Studio tracks** over one shared Creative Project / Element Library / Capability / Job / Creative Graph / QA kernel;
- public Higgsfield behavior was re-audited through its skills plus current Canvas, Popcorn, Cinema Studio, Elements, Soul Cast, and Games/Supercomputer documentation, and the plan now maps those product layers instead of comparing only MCP/CLI skills;
- a third parity pass on 2026-09-13 audited the official Higgsfield MCP landing page/help-center flow and added missing utility/upload/history/cost/job/media-delivery coverage; a fourth 1:1 pass the same day corrected responsibility: Higgsfield's docs say the connected **agent** selects a model automatically, so Masih Awam MCP now exposes semantic capabilities plus opaque compatible execution bindings but owns **no agent/model/provider selection or skill-routing policy**;
- Canvas-style graph execution, SceneBoard/Director **state contracts**, Elements, and Game build/playtest/multiplayer/deploy behavior remain first-class roadmap scope; AI planning/directing, agent choice, model/provider selection, prompt authoring, and fallback policy are explicitly upper-layer concerns;
- Higgsfield remains a public behavioral/product benchmark only, not a runtime dependency;
- game documentation currently shows an evolving command/project surface; the plan therefore freezes behavior contracts rather than copying transient Higgsfield command names;
- no production implementation files were changed as part of these planning reviews; Plan 069 remains plan-only.
