# Plan 069 — Creative Production Platform for Games, Scenes, and Anime

Status: **PLANNED — supersedes the Blender-only and anime-only Plan 069 scope before implementation; no production source implementation has started**

Created: 2026-09-11
Updated: 2026-09-13

## Goal

Build a first-party **Masih Awam Creative Production Platform** that can take one creative brief through reusable Elements, multi-model generation, storyboard/scene direction, Blender production, playable-game construction, animation, visual/temporal/playtest QA, compositing, deployment/export, and reusable project handoff **without depending on Higgsfield accounts, Higgsfield MCP, Higgsfield CLI, Higgsfield APIs, or Higgsfield-hosted generation**.

The product benchmark is Higgsfield's public 2026 operating model, not merely its MCP surface: compact skills plus references, live model/workflow discovery, durable generation jobs, Canvas-style graph composition, reusable Elements, Soul/Soul Cast-style identity, Popcorn-style connected storyboards, Cinema Studio-style directing and cinematography controls, game design/build/multiplayer/deploy workflows, visual QA, surgical revisions, and explicit publish gates. Masih Awam should reproduce those **behaviors and product layers** with its own runtime, state, skills, adapters, Blender integration, coding/game runtime, and operator-owned engines.

Three production tracks are first-class from the architecture stage:

1. **Scene Studio** — script/brief -> Elements -> storyboard -> hero frames -> shot list -> camera/lighting/audio -> generated or Blender-backed cinematic scene.
2. **Anime Studio** — consistent fictional characters/worlds -> turnarounds -> optional 3D/Blender assets -> rig/animation -> multi-shot anime sequence.
3. **Game Studio** — game brief -> design/STYLE FORMULA -> asset manifest -> 2D/3D/audio assets -> playable browser build -> playtest -> optional multiplayer -> deploy, with public publication separate.

The first integrated release still advances incrementally: prove shared creative state and storyboard/scene quality first, then one short anime scene and one small verified browser game. A full anime episode, large game, or fully collaborative studio comes only after those smaller benchmarks pass.

## Success criteria

Plan 069 is successful when Masih Awam can eventually demonstrate all of the following through its own first-party contracts:

1. A user can give a text brief plus references and receive a bounded, mode-aware production interview rather than an immediate uncontrolled generation call.
2. One **Creative Project** owns reusable Elements and manifests for characters, locations, props, style, audio, scenes/shots, game assets, generated outputs, QA, and provenance.
3. Skills describe **how to produce** while tools/adapters describe **what the runtime may do**; skills do not hard-code one provider/model as product architecture.
4. Runtime capability and workflow discovery can report available image/video/audio/3D/DCC/game/build capabilities and validated schemas before a workflow commits to them.
5. Generation/build jobs have first-party create/get/wait/cancel/result semantics, deterministic project ownership, bounded outputs, and retained source metadata.
6. A Canvas-style production graph can compose typed inputs, Elements, generation/edit nodes, storyboard/scene stages, Blender/DCC stages, game-build stages, QA, and delivery with partial reruns and reusable templates.
7. Reusable **Elements** provide stable project-scoped identities for Character, Location, Prop, Style, Audio/Voice, 3D Asset, Animation Clip, and approved media revisions; Elements can be reused across scenes, anime shots, and games.
8. Character identity supports both real-person reference/training adapters when explicitly authorized and **fictional-character creation** comparable to Soul Cast: structured appearance, outfit, archetype/personality/backstory, canonical views, later rig/voice bindings, and cross-scene consistency.
9. A Popcorn-like storyboard layer supports Auto and Manual planning modes, connected multi-frame boards, explicit reference roles, reusable Elements, and continuity of character/location/style/lighting/spatial logic before expensive video or animation work.
10. A Cinema Studio-like Scene Director can convert a script/brief into editable shots with project-global style/lighting/palette rules and per-shot camera/lens/focal/aperture/movement/framing/tempo controls; the agent drafts settings but generation remains an explicit reviewed action.
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

- first-party creative skill architecture and progressive reference/guidance conventions;
- shared Creative Project state, **Element Library**, asset/provenance/revision manifests, and project-scoped reuse;
- capability/model/workflow discovery separate from skill logic;
- provider/engine-neutral generation contracts and local/operator-owned adapter selection;
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
- skill evaluations and scene/anime/game production acceptance fixtures;
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
- reproducing Higgsfield marketing/product-commerce verticals before the shared creative kernel, scene, anime, and game tracks are proven.

## Core product decision

**Blender is not the Higgsfield replacement, and generation engines are not the product.** Blender is one persistent DCC backend; coding/game runtimes are another execution backend; generative engines are interchangeable capability providers. Plan 069 owns the higher-level Creative OS that composes them.

```text
User / Agent / Creative Director
              |
              v
      Masih Awam Creative Skills
      | routing, interviews, guides,
      | style/game/scene methodology
              |
              v
     Creative Project + Element Library
     | Character | Location | Prop | Style
     | Audio | 3D Asset | Animation | Media
              |
              +-------------------+
              |                   |
              v                   v
        SceneBoard / Director   Canvas Graph
        storyboard + shots      typed DAG/templates
              |                   |
              +---------+---------+
                        v
              Capability / Workflow Router
                        |
      +-----------------+------------------+------------------+
      |                 |                  |                  |
      v                 v                  v                  v
 Image/Video Engine  Audio Engine      3D Bootstrap       Blender DCC
      |                 |                  |                  |
      +-----------------+------------------+------------------+
                        |
                        +------------------------------+
                        |                              |
                        v                              v
                Scene / Anime Runtime           Game Build Runtime
                shots, render, composite        code, physics, rooms
                        |                              |
                        +---------------+--------------+
                                        v
                              QA / Playtest / Analysis
                                        |
                                        v
                           Export / Deploy / Publish Gate
```

The important architectural rules are:

> **Skills request capabilities; adapters choose engines.**

> **Elements are reusable creative identity; graphs and manifests describe production; engines only execute.**

> **Scene and game production share assets/state but own different verification loops.**

A creative skill should ask for semantics such as `image.reference_generate`, `storyboard.sequence`, `scene.direct`, `video.image_to_video`, `audio.voice`, `3d.image_to_mesh`, `dcc.character_rig`, `game.build`, `game.playtest`, or `game.deploy`. It must not make the product architecture depend on a transient vendor/model identifier or one game/render engine.

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
| Skills + references | compact trigger/decision skills; detailed on-demand references; explicit route-outs/chaining | first-party creative skills + progressive references + evals | **P0** |
| Generate | one router across image/video/3D/audio, live model schema, jobs, reusable outputs | semantic capability router + adapter discovery + durable creative jobs | **P0** |
| Canvas | node-based infinite production board; any model as node; branching/parallel compare; reusable workflow templates; partial execution | typed Creative Graph runtime first, then Nuxt visual graph editor; curated safe nodes; template save/reuse | **P0/P1** |
| Elements | reusable Characters, Locations, Props, saved outputs/reference media across shots/projects | project Element Library with Character/Location/Prop/Style/Audio/3D/Animation/Media types | **P0** |
| Soul ID | train/reuse a real-person identity across generation paths | optional authorized identity adapters bound to Character Elements, never the source of truth | **P1** |
| Soul Cast | construct fictional actors from structured character dimensions/backstory and reuse them across scenes | Fictional Character Builder -> Character Pack/Element -> canonical views/traits/outfit/personality/rig/voice bindings | **P0/P1** |
| Popcorn | Auto or Manual connected storyboards; multiple references; sequence-level character/light/atmosphere/spatial consistency; frame edits | SceneBoard with Auto/Manual shot planning, multi-frame board, Element references, continuity constraints, revision lineage | **P0/P1** |
| Cinema Studio | hero-frame-first filmmaking; script-to-shot AI Director; reusable Elements; global look controls; per-shot camera/lens/focal/aperture/moves; native audio; long/reference-heavy clips | Scene Director with project-global visual rules + per-shot cinematography schema; agent drafts, user/review gate triggers execution; generated-video and Blender backends | **P1** |
| Video Explainer | style lock, script blocks, voice/video dependency ordering, deterministic assembly | generic sequence DAG and audio-first dependency patterns usable by scenes/anime | **P1** |
| Game Generation / Supercomputer Games | prompt -> game design -> asset manifest/style -> parallel asset generation + code -> local verification -> solo/multiplayer -> deploy -> optional publish | Game Studio owning design, assets, coding, playtest, multiplayer adapter, deploy; publication separate | **P1** |
| Websites/Apps | scaffold/edit/test/deploy full-stack product; generated media can feed app/site | reuse Masih Awam coding agent + future creative-app/project-site templates | **P3** |
| Brandkit / Photoshoot / Cards / Marketing | domain skills lock identity/style and compile specialist deliverables | later vertical skills built on same Element/Graph/QA kernel | **P3** |
| Virality Predictor / analysis | analyze completed media instead of generating | optional `creative.analyze`/audience-quality adapters; not required for core scene/anime/game launch | **P3** |
| Apps/effects/templates | one-click packaged workflows | saved Creative Graph templates/presets with typed inputs and bounded outputs | **P1/P2** |

**Parity claim boundary:** Plan 069 aims for near-parity in **workflow architecture and user-visible production capability** for scenes, anime, and browser games. It does not claim access to Higgsfield's proprietary models, identical visual quality, credit system, private prompts, private marketplace implementation, or exact UI.

## Higgsfield MCP parity matrix — must not be hand-waved into adapters

The official MCP surface is narrower than the whole Higgsfield website but broader than generic image/video generation. Plan 069 must explicitly cover the agent-facing glue below so an external MCP client can complete the same class of workflows without hidden manual handoffs.

| Higgsfield MCP behavior | Masih Awam parity contract | Required behavior |
| --- | --- | --- |
| OAuth connection; no generation API key exposed to the agent | existing Masih Awam MCP OAuth + operator-owned adapter credentials | creative tools inherit authenticated MCP identity; provider secrets never enter model-visible args or generic terminal authority |
| All image/video models reachable through one connector | `creative model list/get` plus semantic capability discovery | concrete model inventory is discoverable at runtime; agent may auto-select or honor an explicit user-selected model when compatible |
| Model parameters available directly through MCP | validated per-model/per-workflow schemas | free parameter selection is bounded by reviewed schema; skills may provide defaults but cannot hide supported user controls |
| Text, reference-image, and mixed-reference generation | typed media/reference roles | one or multiple references can be attached to a job with explicit roles and adapter compatibility validation |
| Image upscaling | `image.upscale` | preserve lineage, requested scale/resolution, bounded output and QA |
| Video upscaling | `video.upscale` | preserve source timing/audio where supported and report adapter limitations honestly |
| Image background removal | `image.remove_background` | transparent/derived asset with parent lineage |
| Video background removal | `video.remove_background` | alpha/matte or equivalent reviewed output contract; duration/resolution bounds |
| Image expand/outpaint | `image.outpaint` | aspect/canvas expansion without overwriting accepted parent revision |
| Video reframe/expand | `video.reframe` | target aspect/resolution with normal job/cost/QA semantics |
| Motion-control generation | `video.motion_control` / typed motion reference | character/reference image and motion video have distinct roles; timing/source metadata retained |
| Reusable Soul characters | Character Elements + optional identity adapter | selected character revision can be referenced by name/ID across jobs without re-uploading source photos |
| Reusable reference Elements for characters, locations, props; several per prompt | Element Library | jobs accept multiple Element IDs/revisions and preserve dependency lineage |
| Voiceover / speech generation | `audio.speech` / `audio.voice` | language/performance metadata, contained output, timing metadata |
| Voice cloning | `audio.voice_clone` | explicit authorized reference/audio ingest, provenance, reusable voice Element, adapter-specific artifact hidden behind Element contract |
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
| Credit balance check | `creative budget/status` | local adapters report configured compute/quota state where measurable; paid adapters report remaining quota/cost data only when their API safely provides it |
| Cost before generation | `creative estimate` | estimate model/workflow cost or local compute class before submit; estimates carry units/source/confidence |
| “Ask before spending” user workflow | enforceable budget/approval policy | support per-job/batch/session/project thresholds and approval gates; unlike Higgsfield's prompt-only cap, hard limits should fail closed where platform policy can enforce them |
| Durable create/get/wait/list semantics | first-party creative job lifecycle | `submit/get/wait/list/cancel`, retained status, bounded failures, stable result assets, cross-turn retrieval |
| Separate workflow catalog from model catalog | `creative workflow list/get` | higher-level chains have schemas/cost inputs/results but still create normal jobs |
| Full skill-based multi-step production through MCP | first-party skill registry + Creative Graph/Scene/Game orchestration | skills may plan several jobs, reuse Elements/assets, run QA, and return final deliverables without requiring the user to manually bridge stages |
| Agent auto-selects a model when user does not specify one | selection policy | selection is based on capability, references, duration/resolution, local resources, quality evals, license/cost policy; selected model is reported in lineage |
| User can force an exact model | explicit model override | honor when active/compatible; otherwise explain precise incompatibility instead of silently substituting |

### MCP parity rules

1. **Model catalog, workflow catalog, asset/history catalog, and skill catalog are different concepts.** Do not collapse them into one `creative_capabilities` blob that makes agent discovery ambiguous.
2. **Every transform is a first-class lineage operation.** Upscale, remove-background, outpaint, reframe, motion control, dubbing, and clip extraction produce child assets rather than overwriting accepted inputs.
3. **External-client upload is part of the product contract.** Conversation-local attachments alone do not satisfy MCP parity because a generic external MCP client may not be able to pass local file bytes directly.
4. **Media return and media persistence are separate guarantees.** A user should see/review the result in the current conversation and still be able to find/reuse it later by Asset ID.
5. **History is queryable state, not log scraping.** Generation/upload history uses project-owned records and typed filters rather than reading raw activity logs.
6. **Cost/budget behavior is explicit.** Local-first does not mean “free”; jobs may consume GPU time, provider quota, disk, or money, and the agent should be able to estimate and obey hard limits before a batch starts.
7. **Utility/edit operations belong to the same job/QA system as generation.** They must not become ad-hoc shell commands or direct engine calls.
8. **Client-specific omissions are not platform architecture.** Higgsfield's ChatGPT plugin currently omits some surfaces such as audio/website building; Masih Awam's core MCP contract should remain client-neutral and let each client expose the subset it can render/authorize.

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
| restore/stabilize/time-remap/auto-cut | post-production utility adapters under the same job/asset contract | P2 |
| color grading/reference color match | scene/style color specification + deterministic/post adapter | P2 |
| Marketing Studio / ad multiplier | vertical skill/template over Elements + Scene/Audio/Graph; no second job/state system | P3 |
| Website Building | existing Masih Awam coding lifecycle + creative Asset/Element imports | P3 except game deploy path already P1 |

Any future extended utility must use the existing model/workflow discovery, safe ingest, Asset lineage/history, budget, jobs, QA, and graph authority. “Parity expansion” is not permission to add ad-hoc provider calls.

## What Higgsfield gets right — the operating model to reproduce

Higgsfield's strongest pattern is not any individual model. It separates **decision knowledge** from **execution capability**:

- compact `SKILL.md` files keep trigger rules, stage flow, decision trees, UX rules, and route-outs near the agent;
- detailed model tables, prompts, troubleshooting, and domain knowledge move into on-demand references;
- skills chain through explicit returned values rather than hidden conversational magic;
- model/workflow schemas are discovered live instead of assuming static parameters forever;
- media has typed roles and validation before submission;
- generation creates jobs that can be waited on/retrieved later;
- domain workflows ask only the questions that materially change the output;
- expensive generation is gated by concept/style decisions;
- specialized workflows can own prompt enhancement so the agent does not freehand every production prompt;
- completed outputs become reusable inputs to later stages;
- final results are visually inspected where possible;
- revisions can be surgical and preserve accepted composition/state;
- deployment/publication is a distinct action from creation and remains explicit.

Plan 069 adopts those product patterns while replacing the backend completely.

## 1:1 operating-model comparison

| Higgsfield behavior | Why it matters | Masih Awam equivalent | Scene / Anime / Game use |
| --- | --- | --- | --- |
| Skill auto-trigger and `Use when` / `NOT for` boundaries | avoids one giant ambiguous agent prompt | first-party creative skills with explicit trigger/route-out contracts | route character design vs shot production vs rigging vs key art correctly |
| Small decision-oriented `SKILL.md`, large on-demand `references/` | controls context cost | keep routing/stage logic compact; load detailed anime references only after a path is chosen | anatomy/proportion, topology, toon shading, facial rig, animation principles loaded only when needed |
| Minimal, mode-specific interviews | gathers only information that changes production | typed intake gates per skill | ask style/character/shot questions once; do not interrogate the user repeatedly |
| Live `model list/get` | prevents stale model assumptions | `creative_capabilities` / adapter schema discovery | know which reference-image, video, audio, 3D, or resolution features are actually available |
| Separate `workflow list/get` from model catalog | treats chains as first-class products | first-party workflow registry separate from engine catalog | turnaround generation, image->3D bootstrap, rigging, lipsync, shot render are workflows, not “models” |
| Media role validation | avoids malformed generation inputs | typed creative asset roles and adapter validation | `character_front`, `character_side`, `style_reference`, `motion_reference`, `voice_reference`, etc. |
| Auto-upload local path / reuse previous job output | lets outputs chain naturally | reviewed workspace materialization + stable asset IDs | concept art -> SceneBoard -> Blender/game asset -> shot/build |
| Canvas graph | keeps a whole multi-model workflow visible, branchable, reusable, and partially rerunnable | typed Creative Graph + saved templates + graph execution state | compare scene looks, branch anime variants, generate game assets in parallel without losing lineage |
| Elements | turns accepted characters/locations/props into reusable project assets instead of repeated prompt text | Element Library with typed references and selected revisions | same hero/location/prop reused by Scene Studio, anime shots, and Game Studio |
| Popcorn Auto/Manual storyboard | sequence consistency is solved before expensive video generation | SceneBoard Auto/Manual planning, connected frames, continuity rules, frame-level revision | direct a cinematic scene, anime board, or game cutscene with shared cast/location/style |
| Cinema Studio AI Director | script/idea becomes editable shot settings rather than an opaque monolithic generation | Creative Director drafts Scene Manifest + cinematography; user/review gate triggers jobs | global style/lighting plus per-shot lens/focal/aperture/move/tempo for generated video or Blender |
| Hero Frame First | locks composition/cast/location/look before motion makes changes expensive | approved hero frame/storyboard frame required before selected high-cost motion paths | cheaper scene/anime iteration and stronger continuity |
| Soul ID reusable identity | consistency survives many generations | Character Identity Pack | authorized real-person identity or recurring visual identity across scenes/media |
| Soul Cast fictional actor builder | invented characters need structured creation, not face training | Fictional Character Builder -> Character Element/Pack | anime/game actors with physique, outfit, traits, archetype/backstory, canonical views and later rig/voice |
| Brandkit reusable identity system | locks visual system before assets proliferate | Style Bible + World Bible | line language, shape language, color script, shader family, environments, typography/key-art rules |
| Domain prompt enhancer | encodes specialist production language | first-party prompt/spec compiler owned by skill/workflow | convert approved character/shot manifest into engine-specific prompt/graph parameters |
| Product Photoshoot mode router | intent chooses workflow, not surface keywords | anime still/key-art mode router | character portrait, full-body key art, action pose, environment still, expression sheet, promo composition |
| Thumbnail concept gate | concept is selected before costly render | shot/key-art concept gate | silhouette/readability/composition checked at thumbnail/storyboard scale before final generation |
| Style preset resolve before explainer blocks | one style key stabilizes multi-shot output | locked Style Bible / Style Pack | all shots share line/shape/palette/material/camera language |
| Generate all narration blocks before video blocks | freezes one modality before dependent generation | dependency-aware shot DAG | lock dialogue/voice timing before lipsync and final animation where appropriate |
| Explicit job `create/get/wait/list` | generation is durable work, not one RPC | first-party creative job lifecycle | long image/video/3D/audio jobs survive normal agent turns and can be referenced later |
| Cost query before workflow | prevents uncontrolled spending | resource/compute estimate where an adapter can provide it | estimate local GPU/runtime or paid third-party cost before large batches |
| Multi-variant generation with controlled dimensions | explores deliberately | variant sets with explicit changed fields | vary pose/camera/expression while identity/style remain locked |
| Post-render visual gate | model output is evidence, not success by assumption | `creative_visual_qa` + model vision/manual review | identity, hands, costume, silhouette, line continuity, text, props, framing |
| Surgical edit from selected job | preserves good state | revision lineage + mask/scope-aware edits | change expression/background/color/camera without resetting accepted character design |
| Skill chaining via returned IDs | keeps boundaries explicit | typed project/Element/asset/job references | Character -> SceneBoard -> Blender/Video -> Anime; Style/Assets -> Game Build -> Deploy |
| Game `STYLE FORMULA` + asset manifest | global coherence before parallel asset/code work | Style Bible + Game Design Manifest + Asset Manifest | no game code/visual batch before core loop, controls, performance budget, style, and assets are frozen |
| Parallel game asset generation + coding | uses idle generation time and treats game build as orchestration | job DAG can run independent image/3D/audio jobs while coding agent builds against stable manifest paths | faster game production without hidden race/asset-name drift |
| Game local verification | a generated build is not “done” until complete loop/input/runtime errors are checked | game playtest contract + browser/runtime verification + two-session multiplayer test where applicable | verify win/lose/restart, keyboard/touch/gamepad, responsive render, fixed-step behavior, missing assets, console errors |
| Deploy separate from publish | a playable private/share URL is not the same as public marketplace publication | `game.deploy` and `game.publish` are different authority/effect boundaries | user may test/share without accidental public listing |
| Website/app create -> repo edit -> deploy | creation and publication are separate lifecycles | reuse coding-agent project/build/deploy lifecycle | game/site/app source stays editable; deploy/publish remain explicit |
| Eval scenarios/version sync | skills remain testable products | creative skill eval suite + versioned contracts | prove routing, state preservation, QA/playtest, scene/anime/game benchmarks over time |

## 1:1 mapping of the nine Higgsfield skills

This is a **behavioral mapping**, not a plan to copy their product names or vendor-specific prompts.

| Higgsfield skill | Core mechanic to learn | Masih Awam native counterpart | Priority for target |
| --- | --- | --- | --- |
| `higgsfield-generate` | broad media router + live schema discovery + jobs + reusable media | `creative-generate` capability/skill | **P0** shared foundation |
| `higgsfield-soul-id` | one-time reusable real-person identity training/reference | `character-identity` adapter binding inside Character Element | **P1** when authorized; not required for fictional characters |
| `higgsfield-brandkit` | lock palette/type/logo/style, dependency-aware revisions | `style-bible` + World/Visual Bible | **P0** shared scene/anime/game visual-system authority |
| `higgsfield-product-photoshoot` | mode router + short interview + specialist prompt enhancer | `creative-stills` / `anime-key-art` / environment reference workflow | **P2** useful production pattern |
| `higgsfield-marketplace-cards` | fixed deliverable bundle from one identity | `promo-pack` templates | **P3** after production core |
| `higgsfield-video-explainer` | style lock + block planning + audio-first dependency + assembly | `scene-sequence` dependency DAG and deterministic assembly | **P1** scene/anime orchestration pattern |
| `higgsfield-youtube-thumbnail` | truthful concept gate + variants + visual QA + surgical edits | `key-visual` / cover workflow | **P2**; validates concept/QA/revision patterns |
| `higgsfield-game-generation` | game profile + STYLE FORMULA + asset manifest + parallel generation/build + playtest + deploy/publish split | **`game-production` / Game Studio** | **P1 first-class target**, not a later spinoff |
| `higgsfield-websites` | scaffold/edit/test/deploy lifecycle and media chaining; public docs now also describe app/game project types | existing Masih Awam coding agent + reviewed web/game project templates | **P1 for game build/deploy**, **P3 for generic sites/apps** |

The first release does **not** need every marketing/business vertical. It does need the shared mechanisms that make Higgsfield feel like one coherent creative platform: skills, Elements, graph/workflow composition, capability discovery, durable jobs, storyboards/scenes, generation, QA/revisions, editable source/project state, game playtest, and explicit deploy/publish boundaries.

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

Generalize Higgsfield **Soul ID + Soul Cast** into one reusable Character Element/Pack. Real-person identity training is only one optional adapter; invented anime/game actors start from structured character design instead.

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
- generation adapter-specific optional identity artifacts such as embeddings/LoRA/checkpoints, stored as implementation details rather than product identity.

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

The Style Pack is engine-neutral. Engine adapters translate it into prompts, node graphs, shader setups, or render configuration.

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

The same scene/shot specification may compile into a generative-video request, Blender camera/animation setup, or a game cutscene adapter. Engine-specific syntax does not belong in the manifest.

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
- cost/compute estimate metadata where adapters support it;
- no arbitrary model-supplied executable node/plugin/ComfyUI graph in the initial release;
- a future Nuxt visual editor can render/edit this same graph contract rather than inventing a second workflow model.

### Audio / Voice Pack

For recurring characters and sequences:

- voice identity/reference or TTS adapter binding;
- language/pronunciation notes;
- emotional/performance direction;
- dialogue timing artifacts;
- SFX/music references;
- licensing/provenance where relevant;
- lipsync/viseme timing output when available.

## Production gates

The workflow must deliberately slow down at cheap decisions and speed up after approval.

### Gate A — Brief

Before generation, determine only missing facts that materially change production:

- what is being made;
- target duration/format;
- character count and which references are authoritative;
- style/reference intent;
- must-preserve details;
- autonomy level: user chooses major creative direction vs agent may choose.

No repeated questions for facts already visible in attachments or conversation.

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

- compile approved Scene/Game/Asset state into engine-specific jobs or editable source;
- start independent jobs in parallel where safe;
- partial reruns must preserve unrelated accepted ancestors;
- agent-generated Director suggestions do not silently trigger expensive generation/publication.

### Gate F — Visual / temporal / structural / playtest review

Never claim production completion from tool exit status alone.

- still/scene/anime work requires the applicable visual/structural/temporal review;
- game work requires local HTTP runtime verification of the complete loop, restart, missing assets, console/runtime errors, input methods, responsive render, timing/performance assumptions, and two-session multiplayer behavior when online/local-multi claims depend on it;
- failed identity/style/deformation/readability/gameplay checks trigger bounded revision rather than automatic full rebuild;
- deploy may happen only after the relevant review passes; public publish remains a separate explicit action.

## Prompt/spec compilation

Higgsfield hides specialist prompt assembly behind some domain workflows. Masih Awam should reproduce the **separation of concerns**, not a private prompt.

The skill produces a structured creative specification. An adapter-specific compiler converts that specification into:

- image/video/audio prompt text;
- reference ordering/roles;
- model parameters;
- ComfyUI/workflow parameters if that adapter is selected;
- Blender script/template arguments;
- deterministic compositor/layout parameters.

Rules:

1. User-facing skill logic owns creative intent and invariants.
2. Adapter compiler owns engine syntax.
3. Raw vendor prompt details are not the durable project contract.
4. A revision should change only the intended structured fields when possible.
5. Prompt/compiler versions must be traceable in generation lineage so a result can be reproduced/explained.

## Capability abstraction

Initial semantic capability vocabulary should cover at least:

```text
model.list
model.get
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

storyboard.auto
storyboard.manual
storyboard.revise_frame
scene.direct
scene.hero_frame
scene.shot_compile
scene.continuity_check

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

game.design
game.build
game.playtest
game.multiplayer_local
game.multiplayer_online
game.deploy
game.publish
```

This vocabulary is a planning target, not a requirement to expose one MCP tool per line. The implementation should keep the public surface compact and allow one tool to advertise multiple semantic capabilities through discovery.

## Generation-engine strategy

Do not recreate Higgsfield by replacing it with another hard-coded cloud vendor.

### V1 decision rule

At implementation time, select the smallest operator-owned media-engine substrate that can satisfy the first scene/anime milestones. **ComfyUI is the preferred media candidate to re-audit** because it can act as a local graph/runtime for multiple image/video/3D workflows, but this plan does not claim it is installed or freeze it without a fresh compatibility/security review. Game source/build/runtime ownership remains with the existing Masih Awam coding/tool workspace plus a reviewed browser-game template/runtime contract; do not force game code execution through the media engine.

Any v1 engine adapter must satisfy:

- disabled by default;
- explicit operator configuration;
- bounded/local or explicitly approved endpoint policy;
- no model-supplied arbitrary endpoint;
- schema/capability discovery;
- curated workflow/template selection rather than arbitrary model-supplied executable graphs in the first release;
- bounded job concurrency/timeouts/results;
- stable project/job/asset lineage;
- credentials, if any, isolated from generic `terminal_exec`;
- clear effect classification for generation that writes workspace outputs or uses external services;
- no claim that every installed third-party custom node is safe merely because the engine is local.

### Engine adapters are replaceable

Future adapters may target local runtimes, user-owned remote GPU workers, or paid providers. Skills must remain unchanged when equivalent capabilities move between adapters.

The runtime must expose **two discovery layers**:

1. semantic capability discovery (`image.reference_generate`, `video.motion_control`, etc.) for durable skill routing; and
2. concrete model discovery (`model.list/get`) for users/agents that want to inspect or force a specific installed/available model.

A model descriptor should expose only bounded non-secret facts needed for routing: stable adapter-local model ID, display name, supported semantic capabilities, media/reference roles, parameter schema/bounds, duration/resolution/aspect constraints, license/usage notes when known, cost/compute estimator availability, and health/availability classification. It must not expose credentials, raw provider config, arbitrary endpoints, or unreviewed executable workflow internals.

Model selection belongs to runtime policy based on:

- required capability;
- reference/identity support;
- resolution/duration constraints;
- latency/resource budget;
- local hardware availability;
- user preference or explicit model override;
- measured quality on scene/anime/game evals;
- license/usage constraints.

If the user names an exact active model and it supports the request, the runtime must preserve that choice. If it is unavailable or incompatible, fail/explain precisely rather than silently substituting. If no model is specified, auto-selection may choose one and must record the choice in job lineage.

Do not select a default because it is trendy or because Higgsfield currently selects it.

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
- adapter ID and adapter version;
- status/progress classification;
- creation/completion timestamps;
- safe parameters summary;
- input asset IDs;
- output asset IDs;
- failure classification without raw credential/provider leakage;
- compute/cost estimate/actual when the adapter can provide it.

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

The graph manifest above is not merely storage. It is the shared orchestration substrate that gives Masih Awam the useful behavior of Higgsfield Canvas while preserving stronger execution boundaries.

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
- `EditImage`, `ReframeVideo`, `UpscaleMedia` where active adapters support them;
- `StoryboardAuto`, `StoryboardManual`, `HeroFrame`;
- `SceneCompile`, `ShotCompile`, `ContinuityCheck`;
- `BlenderInspect`, `BlenderAuthor`, `BlenderRender`, `BlenderExport` as high-level graph stages backed by the reviewed Blender capability;
- `GameBuild`, `GamePlaytest`, `GameDeploy`;
- `VisualQA`, `TemporalQA`, `GameQA`, `UserApproval`;
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

### Scene intake modes

- **Script/brief mode** — parse a paragraph/script into scenes and candidate shots.
- **Auto storyboard mode** — derive a bounded connected board from scene intent and Elements.
- **Manual storyboard mode** — user/agent specifies each frame/shot explicitly.
- **Existing-board mode** — ingest approved frames and convert them into a Scene/Shot Manifest.

### SceneBoard requirements

- multiple Character/Location/Prop/Style references with explicit roles;
- connected frame sequence rather than independent unrelated generations;
- configurable frame count with a conservative product bound selected at implementation time;
- shot size, action, mood, framing, and continuity intent per frame;
- consistent Element revisions, lighting logic, palette, atmosphere, and spatial relationships;
- frame-level edit/revision without discarding unrelated accepted frames;
- board can feed hero-frame generation, generative-video jobs, Blender blocking, or game cutscene production.

### Creative Director behavior

Mirror the useful Cinema Studio contract: the director **suggests and populates settings; it does not silently generate**.

The Director can:

- break scripts/scenes into shots;
- choose/recommend shot size and composition;
- propose camera/lens/focal length/aperture/depth-of-field intent;
- propose camera movement and movement speed;
- apply project-global genre/style/lighting/color rules;
- reason about pacing/tempo and edit points;
- bind Elements to shots;
- detect missing references/assets before execution;
- compile one Scene Manifest into adapter-specific generated-video or Blender instructions.

The user or active autonomy policy reviews the populated plan before expensive/high-effect execution.

### Hero Frame First

For workflows where a still controls subsequent motion, prefer:

1. Scene/Shot intent;
2. approved Elements;
3. storyboard frame;
4. hero-frame candidate(s);
5. visual QA/selection;
6. only then motion/video/Blender animation.

This is not mandatory for every engine, but it is the default when it materially improves cast/location/composition continuity.

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

### Full game workflow

1. Resolve game profile, delivery context, core loop, win/lose/restart/progression, target devices, inputs, performance budget, language, and player-count mode.
2. Freeze one Style Element / STYLE FORMULA and `design/assets` manifest before generated visual batches or broad game implementation.
3. Resolve multiplayer route: solo, local same-screen, or online room/state-sync.
4. Start independent image/3D/audio generation jobs in parallel.
5. Build/edit source against stable manifest paths while jobs run; placeholders are explicit and later replaced through manifest identity, not ad-hoc filenames.
6. For animated 3D assets, validate skeleton/action compatibility; Blender may rig/retarget/create procedural clips where generation adapters are insufficient.
7. Run the game over HTTP/runtime preview, never claim browser behavior from static source inspection alone.
8. Verify complete loop, restart, assets, console/runtime errors, responsive rendering, declared keyboard/mouse/touch/gamepad controls, timing/physics behavior, and performance budget.
9. For multiplayer, verify at least two sessions/clients and room/state synchronization behavior.
10. Deploy only after QA; public publish remains separately authorized.

### Game runtime architecture

Do not hard-code the product to one frontend/game framework before implementation audit. Freeze a small reviewed browser-game template/runtime family based on actual needs:

- simple 2D/canvas/WebGL path;
- 3D web path when needed;
- platform-owned multiplayer room/state-sync module when online play is enabled;
- source remains editable in a normal workspace/repository;
- the coding agent may use ordinary application tooling, tests, browser/runtime preview, and existing safe terminal/build surfaces;
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
12. the coding agent never restarts the live relay/systemd service as part of source implementation.

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

The system may choose:

- image-to-3D bootstrap when an approved adapter supports it;
- manual/scripted Blender blockout when bootstrap quality is inadequate;
- hybrid workflows where generated mesh is only a starting point.

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

Do not skip shared foundations merely because one model can output a flashy clip or one coding model can produce a toy game. After the shared core, Scene/Anime/Game tracks may advance partly in parallel.

### Shared milestones

| Milestone | Deliverable | Must prove before advancing |
| --- | --- | --- |
| C0 | Creative Project + manifests | skills, Elements, assets, scenes/games, provenance, revisions are coherent/versionable |
| C1 | Element Library + safe media ingress | accepted Character/Location/Prop/Style/Media revisions can be reused without hidden chat state |
| C2 | Capability/workflow/job runtime | discovery, durable jobs, result assets, cancellation, bounded effects work without hard-coded vendor names |
| C3 | Creative Graph / Canvas runtime | typed DAG validates, branches, executes parallel nodes, partial-reruns, saves templates, preserves lineage |

### Scene milestones

| Milestone | Deliverable | Must prove before advancing |
| --- | --- | --- |
| S1 | SceneBoard | Auto and Manual modes can produce a connected board with reusable Elements and continuity state |
| S2 | Directed hero frames / shot manifest | global style/light/palette and per-shot cinematography controls are editable and engine-neutral |
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

## Skill architecture

The final names should be frozen after runtime ownership is audited, but the logical first-party skill set should mirror Higgsfield's specialization discipline while targeting our three production tracks.

### `creative-generate`

Equivalent role to Higgsfield Generate:

- route generic image/video/audio/3D requests;
- inspect capability/schema before uncertain calls;
- validate media roles;
- create/wait/retrieve jobs;
- expose result assets, not raw engine internals;
- route scene/anime/game domain work to narrower skills.

### `creative-canvas`

Equivalent role to Canvas workflow composition:

- create/inspect/validate typed Creative Graphs;
- branch and compare alternatives;
- run independent nodes in parallel;
- partial-rerun dirty descendants;
- promote accepted node outputs into Elements/assets;
- save/instantiate templates;
- never use graph composition to bypass underlying tool approvals.

### `element-library`

Equivalent role to Cinema Studio Elements:

- create/update/promote Character, Location, Prop, Style, Media, 3D, Voice, and Animation Elements;
- resolve selected revision for a scene/game;
- show dependency impact before changing a reused Element;
- deliberately import/copy Elements across projects.

### `style-bible`

Equivalent methodology to Brandkit plus scene style presets:

- establish/extend visual system;
- preserve approved palette/shape/line/material/camera/motion rules;
- maintain dependency-aware revisions;
- produce a reusable Style Element/Pack rather than disconnected prompts.

### `character-identity`

Generalized Soul ID + Soul Cast:

- create/update Character Element/Pack;
- ingest authoritative real-person references only when explicitly intended;
- construct fictional characters from structured design/narrative fields;
- produce/curate turnaround/expression references;
- optionally bind adapter-specific identity artifacts;
- bind later 3D rig/voice assets;
- never make one training backend the identity source of truth.

### `scene-board`

Equivalent role to Popcorn:

- Auto mode expands one scene brief into a connected bounded storyboard;
- Manual mode accepts per-frame direction;
- binds Character/Location/Prop/Style Elements;
- keeps continuity constraints explicit;
- supports frame-level revision and hero-frame promotion;
- emits Scene/Shot Manifest state rather than only loose images.

### `scene-production`

Equivalent role to Cinema Studio / AI Director:

- parse a brief/script into scenes/shots;
- populate project-global style/lighting/palette and per-shot camera/lens/focal/aperture/move/tempo controls;
- follow Hero Frame First when useful;
- compile shots to generated-video, Blender, or mixed backends;
- preserve user/review gate before expensive execution;
- assemble previews and run continuity/visual/temporal QA.

### `creative-stills` / `anime-key-art`

Uses Product Photoshoot/Thumbnail methodology:

- classify requested still by production purpose;
- ask a short mode-specific interview;
- concept gate before expensive render;
- controlled variants;
- visual QA;
- selected-result surgical edits.

Initial anime-focused modes may include `character_portrait`, `character_full_body`, `character_action`, `expression_sheet`, `turnaround`, `environment_key_art`, and `episode_key_visual`. Scene/game art modes may extend the same skill without duplicating the QA/revision kernel.

### `anime-production`

Anime specialization over the shared scene/Element/Canvas kernel:

- owns character/style/world manifests;
- coordinates 2D reference, optional 3D bootstrap, Blender cleanup/rig/facial/animation, SceneBoard, shots, audio, QA, composite, and handoff;
- delegates generic still/scene/generation work rather than duplicating it;
- exposes current milestone/blocker.

### `game-production`

Equivalent role to Higgsfield Game Generation / Supercomputer Games:

- resolve game profile/core loop/player/input/performance constraints;
- freeze STYLE FORMULA + Asset Manifest;
- route game-specific sprite/texture/3D/animation/audio generation;
- start independent jobs while coding/build work proceeds;
- preserve existing source architecture during iteration;
- own browser playtest requirements;
- own solo/local/online multiplayer route;
- deploy a verified build;
- require separate explicit action for public publish.

Later non-core skills can add promo packs, generic project sites/apps, marketing verticals, engagement analysis, and one-click effect templates without changing the shared kernel.

## Skill authoring rules learned from Higgsfield

1. Keep each trigger/decision `SKILL.md` compact; target roughly the same “decision logic only” discipline as Higgsfield's ~300-line guidance, without treating 300 as a repository hard limit unless Masih Awam adopts one deliberately.
2. Keep heavy tables, prompt patterns, examples, troubleshooting, and anime production guides in references.
3. Every reference file must be reachable from its owning skill.
4. Skills are self-contained enough to be reasoned about individually.
5. Route-outs/`NOT for` boundaries are mandatory for overlapping creative skills.
6. Chaining passes typed project/asset IDs, not hidden assumptions.
7. Version skill contracts and evaluate routing before changing major defaults.
8. Model names belong in capability/adapters or references, not core skill identity.
9. A skill cannot grant execution authority that the active MCP/tool policy does not provide.
10. External community skills remain reference material until reviewed; first-party scene/anime/game production logic must be owned and maintained in this repository.

## Repository ownership decision to freeze before implementation

The repository already has both `ai-self/skills/` and `.agents/skills/`, plus runtime instruction-loading code. Do **not** duplicate each creative skill in both trees.

Implementation Phase 1 must audit:

- `ai-self/registry.yaml`;
- `ai-self/skills/` conventions;
- `.agents/skills/` conventions;
- `server/infrastructure/ai/subagent-tool.ts` runtime loading;
- current prompt/tool-selection composition;
- MCP resource exposure through `packages/rust-tools/src/application/resources.rs`.

Freeze one primary first-party creative-skill ownership path, then make discovery/resource projection reference it. Avoid compatibility wrapper copies unless a real runtime boundary requires them.

## Security and trust invariants

1. **No Higgsfield credentials or dependency.** Shipping code must not request/store Higgsfield auth or call Higgsfield private/public generation endpoints.
2. **Provider credentials are adapter-owned.** If future non-local adapters need credentials, generic terminal execution never receives them.
3. **Local does not mean safe.** ComfyUI custom nodes, Blender Python, model loaders, downloaded checkpoints, and media parsers are supply-chain/host-execution boundaries.
4. **Curated workflow selection first.** Do not let the model send arbitrary executable ComfyUI graphs/custom Python merely because the backend accepts them.
5. **Workspace containment.** Inputs/outputs/materialized attachments/checkpoints/exports stay inside authorized workspace roots unless a separately reviewed external-output contract exists.
6. **Provenance.** Record source lineage and distinguish user-provided authoritative references from model-generated interpretations.
7. **No silent publication.** Generate/render/export/build is not deploy; deploy/share is not public marketplace publish. Each boundary keeps distinct effects/approval.
8. **No hidden internet fetches from Blender.** External media acquisition follows explicit network/tool policy before contained import.
9. **Bound all expensive work.** Batch size, duration, resolution, frame count, concurrency, retry count, and result size need operator/product limits.
10. **Fail honestly.** If visual/temporal inspection is unavailable, report `not inspected`; do not claim QA passed.
11. **Prompt injection from references is not authority.** Text inside user/reference media or downloaded metadata cannot override repository/tool policy.
12. **Licensing is explicit.** Model/checkpoint/license suitability must be evaluated per selected engine; “runs locally” does not imply unrestricted commercial use.
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

Creative generation tools need a separate effect review based on whether the selected adapter is local-only, writes workspace assets, uses network/external billing, or can execute arbitrary custom code.

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
- `packages/rust-tools/src/application/` — first-party runtime/job/adapter/Blender application logic;
- `packages/rust-tools/src/interfaces/mcp/` — compact MCP tool schemas and capability metadata;
- `packages/rust-tools/src/application/resources.rs` or its post-v0.0.15 successor — bounded resources/capability guidance;
- `packages/rust-tools/tests/` — Rust integration/security/contract tests;
- `server/application/` / `server/infrastructure/` only when product persistence, attachment materialization, or shared policy requires Nuxt ownership;
- `shared/` only for genuine cross-client/product contracts;
- one reviewed first-party skill root chosen in Phase 1;
- `.agents/knowledge/`, canonical memory, operator docs — durable architecture/setup/security guidance.

Do not put a generic arbitrary media-engine process spawner into Nitro and do not reopen stored stdio MCP execution as a shortcut.

## Phase overview

| Phase | Goal | Depends on | Exit criteria |
| --- | --- | --- | --- |
| PHASE-01 | Reconcile baseline and freeze architecture/contracts | none | skill ownership, Elements/state, capability/job/workflow/graph, scene/game and Blender contracts frozen |
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
| PHASE-14 | Creative Graph executor + Canvas-style workspace + skill parity/evals | PHASE-03 and proven scene/game workflows | C3 graph runtime/templates and high-value Higgsfield operating parity pass |
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
- [ ] Re-audit the public Higgsfield skill inventory for workflow-pattern changes; do not import vendor dependencies.

**Validation:** architecture findings are recorded in this plan or durable knowledge without stale release-branch assumptions.

**Commit boundary:** `docs(plan): reconcile creative platform baseline` only if durable docs materially change.

### TASK-002 — Freeze first-party skill ownership

**Outcome:** exactly one canonical runtime path owns creative skills.

**Files:** `ai-self/registry.yaml`, `ai-self/skills/`, `.agents/skills/`, `server/infrastructure/ai/subagent-tool.ts`, relevant prompt/tool-selection modules.

**Steps:**

- [ ] Map current loading/discovery precedence.
- [ ] Choose one first-party creative skill root.
- [ ] Define how skill references are loaded progressively.
- [ ] Define route-out and chaining metadata.
- [ ] Define version/eval ownership.
- [ ] Reject duplicate mirrored skill folders unless runtime compatibility proves unavoidable.

**Validation:** a test/inspection can identify one authoritative source for a creative skill and its references.

**Commit boundary:** `refactor(skills): establish creative skill ownership` during implementation.

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

### TASK-004 — Freeze MCP-facing model, workflow, budget, asset, and job contracts

**Outcome:** semantic capabilities, concrete model discovery, workflow discovery, budget/cost preflight, asset/history access, job lifecycle, effect policy, and result-media delivery are specified independently from engines.

**Steps:**

- [ ] Freeze semantic capability vocabulary needed through C3/S3/A6/G5 plus MCP utility parity: upscale, background removal, outpaint/reframe, motion control, clip extraction, voice clone/change/dub.
- [ ] Freeze `capability list/get` representation for durable skill routing.
- [ ] Freeze concrete `model list/get` representation and explicit-model override semantics separately from semantic capabilities.
- [ ] Freeze workflow `list/get` separately from model inventory.
- [ ] Freeze asset/upload/history `list/get/search` filters, source tags, stable IDs, and media-result representation.
- [ ] Freeze secure external-client upload request/complete and bounded URL-import contracts.
- [ ] Freeze cost/compute estimate plus global/project/session/job budget-status and hard-limit semantics.
- [ ] Freeze job submit/get/wait/list/cancel behavior and cross-turn retrieval.
- [ ] Freeze result and failure bounds, including current-turn preview/resource delivery plus durable Asset registration.
- [ ] Freeze external-cost/network vs local-compute effect classifications.
- [ ] Decide whether existing relay Tasks can own creative long-running jobs directly or need a thin creative domain layer over the same manager.

**Validation:** mock adapters can advertise different engines/models for the same semantic capability without changing skill contracts; the same fixture can auto-select a model, honor an explicit compatible model, preflight cost, submit, list/retrieve the job, and reuse the returned Asset ID.

**Commit boundary:** `feat(creative): define capability and job contracts`.

**Phase exit criteria:**

- [ ] no implementation depends on a Higgsfield contract;
- [ ] one skill root is authoritative;
- [ ] creative state schemas are frozen for initial milestones;
- [ ] adapter/job semantics reuse existing platform primitives where possible;
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

### TASK-008 — Add semantic capability discovery plus concrete model/workflow discovery

**Outcome:** clients/skills can discover durable semantic capabilities while power users/agents can separately list/get concrete active models and workflows with validated schemas.

**Steps:**

- [ ] Add operator-disabled-by-default creative capability group.
- [ ] Expose active semantic capabilities independently from concrete models.
- [ ] Expose bounded `model list/get` descriptors: adapter-local ID, display name, capabilities, reference roles, validated parameter schema/bounds, duration/resolution/aspect constraints, availability/health, known license notes, and cost-estimator support.
- [ ] Expose workflow IDs and `workflow list/get` separately from models.
- [ ] Support automatic model selection when unspecified and explicit user model override when compatible.
- [ ] Return precise incompatibility/unavailable diagnostics instead of silent substitution.
- [ ] Keep endpoint/credential/raw engine implementation metadata hidden.
- [ ] Return activation/setup hint when disabled.

**Validation:** mock semantic capabilities remain stable while underlying model inventory changes; explicit compatible model selection is preserved and incompatible override fails with a bounded reason.

**Commit boundary:** `feat(creative): expose capability discovery`.

### TASK-009 — Add first-party creative job lifecycle, cost preflight, and enforceable budgets

**Outcome:** generation/transform work can be estimated, approved when required, started, polled, waited, listed, cancelled, and retrieved with stable project ownership and bounded spend/compute authority.

**Steps:**

- [ ] Reuse existing job/task manager lifecycle and cancellation where possible.
- [ ] Add `cost.estimate`/compute-estimate semantics before submit when the adapter/workflow can provide meaningful data.
- [ ] Add `budget.status` for configured provider quota/credits, local compute class/quota, disk/output bounds, and project/session/job hard limits where measurable.
- [ ] Enforce operator/user-configured thresholds before expensive jobs/batches; approval cannot override an operator hard maximum.
- [ ] Add domain metadata only where creative workflows need it.
- [ ] Support submit/get/wait/list/cancel and cross-turn retrieval.
- [ ] Bound concurrent jobs/batches and retry count.
- [ ] Persist/recover only if current platform task semantics cannot satisfy cross-turn retrieval safely.
- [ ] Keep failure/cost/provider diagnostics redacted/classified.

**Validation:** fake adapters cover queued/running/completed/failed/cancelled, list/retrieve after client reconnect, estimate-before-submit, approval threshold, hard-budget denial, timeout, output bounds, and owner isolation.

**Commit boundary:** `feat(creative): add generation job lifecycle`.

### TASK-010 — Add curated workflow registry and Creative Graph contract

**Outcome:** multi-step workflows are discoverable independently from individual engine/model capabilities, and a typed graph can represent their composition before a visual Canvas/executor is added.

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

- [ ] Define workflow schemas independently from adapter/model catalogs.
- [ ] Define typed Creative Graph node/edge/port/revision contract around workflows/jobs/Elements.
- [ ] Define dirty-descendant and partial-rerun semantics.
- [ ] Define template input/output contract.
- [ ] Validate that graph representation cannot encode arbitrary code/endpoints/credentials as ordinary nodes.

**Validation:** workflows advertise required inputs/output roles and reject unsupported parameters before execution; representative scene/anime/game graphs validate without containing engine-specific secrets or arbitrary executable payloads.

**Commit boundary:** `feat(creative): add workflow and graph contracts`.

**Phase exit criteria:**

- [ ] semantic capability, concrete model, workflow, asset/history, and budget discovery are distinct and queryable;
- [ ] explicit compatible model selection and auto-selection both work without changing skill contracts;
- [ ] core MCP utility workflows are represented: upscale, background removal, outpaint/reframe, motion control, clip extraction, voice clone/change/dub;
- [ ] job lifecycle is bounded, listable/retrievable across normal turns, and owner/project scoped;
- [ ] cost/compute preflight and hard budget thresholds can block a batch before execution;
- [ ] graph/workflow contracts can express scene/anime/game dependencies;
- [ ] skills can make routing decisions without hard-coded provider IDs.

# PHASE-04 — Initial reference-aware media generation

**Goal:** prove the reference/identity/style generation substrate through the anime A1/A2 still benchmark before solving full 3D production, while keeping the adapter generic enough for scene/game art.

**Dependencies:** PHASE-02, PHASE-03.

### TASK-011 — Select and secure the initial image/media adapter and core image utility surface

**Outcome:** one reviewed adapter can perform reference-aware image generation/editing plus the image-side utility operations exposed by the MCP parity matrix.

**Steps:**

- [ ] Re-audit ComfyUI as preferred candidate and compare against direct/local alternatives.
- [ ] Confirm operator setup, endpoint/loopback policy, executable/custom-node trust model, result format, cancellation, and model/license constraints.
- [ ] Choose the smallest adapter that satisfies the A1/A2 still/turnaround quality gate while remaining reusable by scene/game art.
- [ ] Implement/route the supported image capabilities through the same job/lineage contract: generate, reference-generate, edit/inpaint, upscale, remove-background, and outpaint; pose/depth controls remain capability-gated when the chosen adapter supports them.
- [ ] Make model list/get schema expose which utility/reference roles each concrete model/workflow actually supports.
- [ ] Keep arbitrary model-supplied workflow graphs disabled in v1.
- [ ] Add operator config/capability activation docs.

**Validation:** safe mock/fixture tests plus operator-only manual smoke cover reference generation and at least one child-asset transform for upscale, background removal, and outpaint; no Higgsfield dependency.

**Commit boundary:** `feat(creative): add initial image generation adapter`.

### TASK-012 — Implement structured prompt/spec compiler

**Outcome:** Character Pack + Style Pack + requested mode compile into adapter inputs with traceable versioning.

**Steps:**

- [ ] Define engine-neutral generation spec.
- [ ] Define reference role/order.
- [ ] Compile only reviewed fields.
- [ ] Record compiler version in job lineage.
- [ ] Support controlled changed-fields for variants.

**Validation:** same creative spec can be serialized without adapter model names; adapter compiler fixture produces deterministic validated parameters.

**Commit boundary:** `feat(creative): compile structured generation specs`.

### TASK-013 — Implement Character Pack + Style Pack still workflow

**Outcome:** generate/curate a stable set of front/side/back/three-quarter/expression references for one anime character.

**Steps:**

- [ ] Build the minimal intake gate.
- [ ] Produce candidate style directions cheaply.
- [ ] Lock selected Style Pack.
- [ ] Produce canonical front/reference view.
- [ ] Produce additional views/expressions with locked identity/style fields.
- [ ] Label generated hidden views as interpreted.
- [ ] Run visual QA and bounded retries.
- [ ] Promote accepted revisions into the Character Pack.

**Validation:** A1/A2 fixture shows recognizable identity/style consistency across the selected set and records QA findings honestly.

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

### TASK-015 — Add optional identity adapters without replacing Character Pack authority

**Outcome:** embeddings/LoRA/fine-tunes or future identity mechanisms can improve consistency while remaining adapter-specific accelerators.

**Steps:**

- [ ] Measure reference-only baseline first.
- [ ] Add training only when evals show a meaningful gap.
- [ ] Store training artifact/license/version lineage.
- [ ] Keep Character Pack references and design constraints authoritative.

**Validation:** deleting/changing one adapter-specific identity artifact does not erase the character's project identity contract.

**Commit boundary:** `feat(anime): support pluggable identity adapters`.

### TASK-016 — Add World/Location Pack

**Outcome:** recurring locations/environment language can be reused across key art and shots.

**Validation:** two shots can reference the same location pack while varying time/camera without losing core location identity.

**Commit boundary:** `feat(anime): add reusable world packs`.

**Phase exit criteria:**

- [ ] style/character/world state survives multiple jobs;
- [ ] adapter training is optional, traceable, and replaceable;
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
- [ ] Choose image-to-3D bootstrap only when active capability and eval quality justify it.
- [ ] Otherwise create/manual-script blockout from references.
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

### TASK-026 — Add audio/voice adapter with MCP parity for speech, cloning, conversion, and dubbing

**Outcome:** reviewed local/operator-owned or explicitly configured adapters can create speech/voice/music/SFX and, where activated, perform authorized voice cloning, voice change, and video dubbing through one contained audio lineage model.

**Steps:**

- [ ] Freeze `audio.voice|speech|music|sfx|voice_clone|voice_change|video_dub` contracts and media/reference roles.
- [ ] Preserve voice/source/license/consent provenance and distinguish generated fictional voices from authorized reference-voice derivatives.
- [ ] Bind reusable accepted voice state to an `audio_voice` Element rather than exposing adapter-specific training IDs as the project identity.
- [ ] Keep credentials/training artifacts isolated if a non-local provider is supported.
- [ ] Produce contained audio/video child assets plus timing/language/voice metadata needed for animation and scene workflows.
- [ ] Keep dubbing/voice conversion independently revisable from body animation or source video.

**Validation:** fixtures cover bounded speech plus mocked/activated clone, voice-change, and dub contracts; unauthorized/missing reference inputs fail closed and project/parent lineage is preserved.

**Commit boundary:** `feat(audio): add anime dialogue capability`.

### TASK-027 — Add pose/action workflow

**Outcome:** agent can create a short coherent action using inspected rig state, API docs, checkpoints, and preview.

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

# PHASE-09 — Scene Studio: SceneBoard, Director, and shot orchestration

**Goal:** reach S1/S2 and produce an S3 candidate: a reusable-Element storyboard/shot workflow comparable in operating behavior to Popcorn + Cinema Studio, usable by both ordinary cinematic scenes and anime.

**Dependencies:** PHASE-04, PHASE-05. PHASE-06/08 are required only for Blender-backed animated shots; generated-video scenes may use another active adapter.

### TASK-030 — Implement SceneBoard Auto/Manual + Scene/Shot Manifest skill

**Outcome:** an approved brief/script becomes a connected board and bounded scene/shot plan before render-heavy work starts.

**Steps:**

- [ ] Implement `auto` mode: one scene brief -> bounded connected storyboard beats.
- [ ] Implement `manual` mode: explicit per-frame/shot direction.
- [ ] Bind Character/Location/Prop/Style Elements with selected revisions.
- [ ] Track global scene look/lighting/atmosphere/spatial constraints across frames.
- [ ] Allow frame-level revision while preserving unrelated accepted frames.
- [ ] Promote selected frames to hero-frame candidates.
- [ ] Freeze shot IDs/durations/assets/continuity/camera intent.
- [ ] Validate required upstream Elements/assets exist.
- [ ] Keep user approval/autonomy behavior explicit.

**Validation:** a scene fixture can produce and revise a connected board without hidden chat context; changing one frame preserves the other accepted frame identities/lineage.

**Commit boundary:** `feat(scene): add sceneboard and shot planning`.

### TASK-031 — Implement Creative Director + generated-video utility surface + shot execution DAG

**Outcome:** script/scene intent is compiled into editable global project settings and per-shot cinematography, while generated-video backends expose the same reusable transform/job semantics expected from the MCP parity matrix.

**Steps:**

- [ ] Add global scene settings: genre/look, Style Element, lighting, color palette, atmosphere, era/time where relevant.
- [ ] Add per-shot settings: shot size/framing, camera profile, lens/focal/aperture/DoF intent, move, movement speed/stabilization, tempo/edit intent.
- [ ] Implement Hero Frame First route when the selected backend benefits from it.
- [ ] Compile a shot to generated-video, Blender, or mixed backend without changing the engine-neutral manifest.
- [ ] Route active video capabilities through the normal model/workflow/job/asset system: text/reference/image-to-video generation, extend, reframe, upscale, remove-background, and motion-control when supported.
- [ ] Validate distinct media roles for motion-control inputs (character/reference image vs motion-reference video) and preserve timestamp/duration lineage.
- [ ] Expose video utility workflows outside Scene Studio too; a user should be able to reframe/upscale/remove-background an existing Asset without manufacturing a Scene Manifest.
- [ ] Director suggestions populate manifests/prompts but do not silently trigger generation.
- [ ] One failed shot/utility transform invalidates/retries only its affected descendants.

**Validation:** same Scene Manifest fixture can compile to at least a fake generated-video adapter and fake/real Blender path with consistent Element/camera intent; separate fixtures prove reframe, upscale, background removal, and motion-control child-asset lineage/job semantics.

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

- [ ] S1 SceneBoard Auto/Manual behavior works;
- [ ] S2 global/per-shot Director controls are represented independently from engines;
- [ ] one 10–30 second S3 candidate has bounded shot state;
- [ ] reusable Character/Location/Prop/Style Elements survive shot changes;
- [ ] generated-video and/or Blender preview can be inspected and revised shot by shot;
- [ ] active video utilities (reframe/upscale/remove-background/motion-control where supported) are independently callable through normal model/workflow/job/asset contracts.

# PHASE-10 — Unified visual/temporal QA and surgical revisions

**Goal:** make evidence-driven revision a shared platform contract rather than one skill's ad-hoc behavior.

**Dependencies:** PHASE-04, PHASE-08.

### TASK-033 — Implement QA finding schema

**Outcome:** still, 3D, render, and temporal review emit bounded domain findings with severity and source revision.

**Validation:** findings distinguish hard fail, soft finding, and not-inspected.

**Commit boundary:** `feat(creative): add production qa findings`.

### TASK-034 — Implement visual QA workflow

**Outcome:** selected outputs are inspected against Character/Style/Shot invariants before promotion.

**Validation:** anime identity/style/text/composition seeded failures are detected or explicitly reported as uninspected.

**Commit boundary:** `feat(creative): add visual qa workflow`.

### TASK-035 — Implement temporal QA workflow

**Outcome:** frame samples/playblast metadata are reviewed against animation/continuity/audio constraints.

**Validation:** seeded foot-slide/timing/identity-drift fixture produces bounded findings.

**Commit boundary:** `feat(creative): add temporal qa workflow`.

### TASK-036 — Implement surgical revision lineage

**Outcome:** a user/agent can change one approved property while preserving other locked production state.

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

### TASK-040 — Implement `game-production` design and manifest workflow

**Outcome:** a game request becomes a frozen Game Design/Build Manifest and STYLE FORMULA before broad asset generation or coding.

**Steps:**

- [ ] Resolve design-only/assets-only/build/deploy intent.
- [ ] Freeze genre, perspective, target devices, core loop, verbs, win/lose/restart/progression, player count, controls, camera, language, physics/timing, and performance/asset budgets.
- [ ] Freeze solo/local/online multiplayer route.
- [ ] Bind shared Character/Location/Prop/Style Elements where applicable.
- [ ] Write stable `design/assets` roles/paths before generated asset batches.
- [ ] Define placeholder policy and missing-asset behavior.
- [ ] Keep public publish outside the build/deploy workflow.

**Validation:** G1 fixture can be built from the manifest without hidden chat assumptions; implementation is blocked when required gameplay/style/input facts remain unresolved.

**Commit boundary:** `feat(game): add game production manifest`.

### TASK-041 — Implement parallel game asset generation and source integration

**Outcome:** image/3D/audio generation jobs can run independently while editable game source is built against stable manifest identities.

**Steps:**

- [ ] Add game-specific media roles for sprites, UI, tileables, sky/environment, textures, 3D models, animation clips, music, SFX, and voice where needed.
- [ ] Start independent generation jobs concurrently under existing job bounds.
- [ ] Build source against manifest-stable placeholders/paths rather than ephemeral job filenames.
- [ ] Promote accepted results into Elements/assets and replace placeholders deterministically.
- [ ] Validate skeleton/action compatibility for animated 3D assets; route to Blender/retargeting when necessary.
- [ ] Preserve source and asset lineage separately.

**Validation:** a fixture can replace delayed generated assets after code exists without manual filename edits or source-wide rebuilds unrelated to those assets.

**Commit boundary:** `feat(game): integrate generated production assets`.

### TASK-042 — Implement reviewed browser-game source/runtime templates

**Outcome:** the coding agent can create/edit a normal workspace project for a small 2D or 3D browser game without depending on an opaque hosted builder.

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

**Outcome:** follow-up prompts amend an existing game rather than silently regenerating it from scratch.

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

# PHASE-14 — Creative Graph executor, Canvas workspace, and skill parity evals

**Goal:** complete C3 and the highest-value Higgsfield operating parity by turning the graph contract into a safe reusable executor/workspace over proven scene/anime/game primitives.

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

**Outcome:** common workflows can be saved and instantiated with new Elements/inputs similar to Canvas templates/Apps without embedding vendor-specific model identities as the product contract.

Initial templates may include:

- character turnaround;
- SceneBoard -> hero frame -> video preview;
- script -> Scene Director -> multi-shot scene;
- anime character -> Blender -> rig -> animation preview;
- game design -> parallel asset batch -> build -> playtest;
- promo/key-visual bundle.

**Validation:** one template is instantiated with materially different Character/Location/Style Elements and preserves graph topology while recompiling adapter-specific parameters.

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

### TASK-050 — Add first-party creative skill and parity eval scenarios

**Outcome:** routing and workflow decisions for Generate/Canvas/Elements/SceneBoard/Scene Director/Anime/Game are testable rather than preference-driven.

Eval categories include direct trigger, natural paraphrase, route-out, minimal interview, user autonomy, unavailable capability, auto-vs-explicit model selection, upload-handoff vs URL-import vs prior-Asset reuse, generation/upload history lookup, estimate-before-generate and budget denial, utility-transform routing (upscale/background removal/outpaint/reframe/motion control/clipper/dub), Element preservation, graph chaining, SceneBoard Auto/Manual choice, Director suggestion-vs-execution gate, game design lock, QA/playtest hard failures, surgical edits, deploy-vs-publish, and no authority escalation through templates.

**Validation:** eval runner produces deterministic routing/contract results without requiring expensive inference for every scenario.

**Commit boundary:** `test(skills): cover creative platform parity`.

### TASK-051 — Add lower-priority reusable delivery templates

**Outcome:** prove the shared kernel can support Higgsfield-like packaged workflows without distracting from Scene/Anime/Game core.

Candidate templates/skills after core parity:

- anime/game promo pack;
- key visual/cover/thumbnail bundle;
- project/portfolio site using existing coding agent;
- optional audience/engagement analysis adapter;
- other reviewed one-click creative effects.

**Validation:** each candidate reuses Elements/Graph/QA/runtime state and has explicit route-out and publish/deploy semantics; none introduces a second identity/workflow system.

**Commit boundary:** feature/skill commits only for selected post-core templates.

**Phase exit criteria:**

- [ ] C3 graph execution/partial rerun/template behavior passes;
- [ ] visual Canvas edits the same underlying graph contract;
- [ ] core creative skill parity is evaluated deterministically;
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

**Validation:** without source-code special cases, materially different fixtures can reach at least S1/S2, A1/A2, and G1/G2 using the same Elements/Graph/skill contracts; at least one track reaches its repeatable-template milestone.

### TASK-056 — Run security and failure matrix

**Outcome:** re-test path escapes, protected credentials, engine endpoint policy, arbitrary workflow/code/template injection, graph authority composition, job ownership/cancellation/output bounds, Blender host authority, malicious media metadata, game networking isolation, multiplayer cross-room isolation, build/deploy/publish separation, and production deployment authority.

**Validation:** relevant focused Rust/Nuxt/browser/runtime tests plus security review pass.

### TASK-057 — Run generic external MCP parity acceptance

**Outcome:** prove the creative platform is usable as a real MCP creative connector from a generic authenticated external MCP client, not only from Masih Awam's own UI or a local coding shell.

Acceptance journey:

1. connect through the normal Masih Awam OAuth-protected MCP endpoint with no creative-provider key exposed to the client;
2. inspect semantic capabilities, concrete models, and workflows separately;
3. query budget/cost status and preflight one small generation;
4. create an external-client upload handoff (or use a safely imported URL) and resolve it to an Asset ID;
5. submit/wait/retrieve one bounded generation and receive both an in-conversation media/resource result and durable Asset ID;
6. run at least one core utility transform on that Asset (for example upscale, background removal, outpaint/reframe as appropriate) and verify child lineage;
7. list/search recent generations/uploads and reuse a previous Asset or Element directly in a second job;
8. run one first-party multi-step skill/graph workflow through the same connector surface;
9. verify an explicit compatible model choice is honored and an incompatible model choice fails precisely;
10. verify a configured hard budget/approval threshold prevents an over-budget request before engine execution.

**Validation:** the flow requires no Higgsfield account/CLI, no local shell command, no arbitrary host path, and no manual download/re-upload between jobs. Client-specific rendering differences are allowed, but the MCP contracts and stable IDs remain identical.

### TASK-058 — Repository closure

**Steps:**

- [ ] Run focused subsystem tests while iterating.
- [ ] Run `pnpm guardrail:fast` before checkpoint commits.
- [ ] Run affected full Rust/Nuxt gates as required by changed ownership.
- [ ] Run browser/runtime game acceptance where game behavior changed.
- [ ] Run `pnpm guardrail:full` before closure.
- [ ] Run dependency/security audits when dependency changes justify them.
- [ ] Update operator docs, architecture/security docs, skills/resources, canonical memory, and this plan's status/checklists truthfully.
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
- semantic capability discovery independent from adapter/model names;
- concrete model list/get schema, explicit compatible override, and precise incompatible/unavailable failure;
- workflow registry remains separate from model inventory and exposes validated inputs/cost-estimator metadata where available;
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

### Engine adapter and MCP utility tests

Use fake/local mock adapters where possible to verify:

- request parameter validation and per-model schema/bounds;
- media-role validation including multi-reference and motion-control image/video role separation;
- auto model selection vs explicit model override;
- image generate/reference/edit/inpaint/upscale/remove-background/outpaint contracts;
- video generate/reference/image-to-video/extend/reframe/upscale/remove-background/motion-control contracts;
- audio speech/music/SFX plus authorized voice-clone/voice-change/video-dub contracts;
- long-video clip extraction preserves source timestamps/lineage and uses only contained/imported source Assets;
- timeout/cancellation/retry bounds;
- output bounds and current-turn media result delivery;
- durable result registration/history reuse;
- cost-estimate and budget-admission behavior;
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

### SceneBoard / Director tests

Verify with deterministic fixtures:

- Auto and Manual storyboard modes produce bounded Scene/Shot state;
- Element references and selected revisions resolve consistently across frames;
- frame-level revision preserves unrelated accepted frames;
- global scene direction and per-shot cinematography stay separate;
- Director suggestions do not auto-trigger generation;
- one Scene Manifest can compile to fake generated-video and Blender backends without changing source-of-truth state;
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

### Skill routing evals

Cover Generate/Canvas/Elements/SceneBoard/Scene Director/Anime/Game triggers and route-outs, minimal interviews, style/identity/game-design gates, capability-unavailable behavior, job/graph chaining, QA/playtest behavior, revision scope, Director suggestion-vs-execution, and deploy-vs-publish without requiring live expensive inference for every scenario.

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
- **Engine adapter outage:** retain project/job state; fail with capability-specific diagnosis; do not silently switch to an adapter with materially different semantics/cost without policy/user approval.
- **Capability unavailable:** report the missing operator capability/setup; continue independent planning/state work where possible.
- **Unsafe media/path:** reject before engine/Blender/game runtime sees it.
- **Skill/template regression:** roll back skill/reference/template change independently from engine/runtime where contracts permit.
- **Model quality regression:** change adapter/model selection policy behind semantic capabilities; do not rewrite Scene/Anime/Game skills.
- **Long-form/large-game failure:** fall back to the last passing track milestone; never broaden from a failing short scene or incomplete core loop to a full episode/large game.

## Alternatives rejected

### Rewrite Blender into a “Higgsfield clone”

Rejected because Blender is a DCC/execution backend, while Higgsfield's valuable architecture lives above execution: skills, state, routing, workflows, QA, and job orchestration.

### Proxy or wrap Higgsfield MCP/CLI

Rejected because the user explicitly requires no Higgsfield account/runtime dependency, and it would keep auth, backend availability, billing, and model behavior outside Masih Awam ownership.

### Copy Higgsfield skill text wholesale

Rejected. The public skills are useful reference material, but Masih Awam needs first-party scene/anime/game methodology aligned to its own runtime/security contracts. Learn the structure/patterns; author and evaluate the actual product guidance here.

### Put every creative operation behind `terminal_exec`

Rejected because credentials, effects, long-running jobs, media results, capability discovery, and Blender host authority need typed first-class boundaries.

### Expose arbitrary ComfyUI workflow JSON from the model

Rejected for the first release because custom nodes/workflows can become a broad execution/supply-chain boundary. Prefer curated reviewed workflow IDs plus validated parameters.

### Add hundreds of Blender atomic tools

Rejected because a compact inspection/preview/I/O/recovery surface plus privileged `bpy` authoring and strong guidance gives more production flexibility with a smaller contract.

### Train a character model before proving reference-based consistency

Rejected as premature complexity. Character Pack + reference workflow is the baseline; training is added only when measured evals justify the cost/complexity.

### Target a full anime episode or large game first

Rejected because scale hides failures across Elements/identity, scene direction, 3D asset readiness, rigging, motion, continuity, gameplay loop, inputs, networking, rendering, and audio. Short Scene S3, Anime A6, and Game G3/G5 benchmarks are the first integrated quality gates.

## Risks

### RISK-01 — Scope explosion

Creative production spans media generation, scenes, Blender/anime, browser games, graphs, and deployment. Mitigation: one shared C0–C3 kernel, then independently gated Scene S1–S4, Anime A1–A8, and Game G1–G6 ladders; marketing/site/analysis verticals remain post-core.

### RISK-02 — Model quality changes faster than skills

Mitigation: semantic capabilities and adapter-specific compilers keep model IDs out of core skill logic.

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

### RISK-08 — Giant context from creative manuals

Mitigation: compact routing skills + progressive reference loading, following the strongest Higgsfield skill-structure pattern.

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

Mitigation: platform-owned bounded room/state-sync and deployment adapters own credentials/infrastructure; generated game source receives only the narrow runtime interface it needs and never generic operator credentials.

### RISK-15 — MCP parity media ingest becomes a host/network escape hatch

Mitigation: external-client upload uses expiring owner/project-bound first-party handoffs; URL import reuses SSRF/redirect/DNS/content-type/size policy; arbitrary local paths and engine-side downloads remain forbidden; all successful ingress resolves to a contained Asset ID before downstream use.

### RISK-16 — Model/catalog parity leaks provider internals or silently changes user intent

Mitigation: concrete model descriptors expose only bounded routing/schema/license/cost facts, never credentials/endpoints; explicit compatible model overrides are preserved, while incompatible requests fail with a precise reason instead of silent substitution.

### RISK-17 — Budget checks stay prompt-only and fail to constrain batch execution

Mitigation: estimate/admission happens in the execution layer, operator hard maxima cannot be overridden by the model, and batch/graph execution accounts for aggregate budget before scheduling descendants.

## Final acceptance criteria

Plan 069 implementation is complete only when:

1. Masih Awam has no runtime dependency on Higgsfield services, auth, CLI, MCP, proprietary model IDs, or hosted state.
2. The first-party operating model covers the high-value Higgsfield patterns: compact skills/references, Generate-style routing/discovery/jobs, reusable Elements, Canvas-style graphs/templates, Popcorn-style Auto/Manual storyboards, Cinema-Studio-style Director/cinematography/Hero-Frame workflow, Soul-Cast-like fictional-character creation, visual/temporal QA, scoped revisions, and Game-Studio-style build/playtest/deploy lifecycle.
3. The parity claim remains bounded to **workflow architecture and production capability**; docs never claim identical proprietary model quality, private prompts, Higgsfield credit economics, marketplace implementation, or pixel-identical UI.
4. One canonical first-party creative skill ownership path exists and Scene/Anime/Game skills route cleanly without duplicate authority.
5. Creative Project, Element Library, Character, Style, World/Location, Asset, Scene/Shot, Game Design/Build, Audio, Creative Graph, QA/Playtest, and revision/provenance state are versioned, contained, and resumable.
6. Elements support at least Character, Location, Prop, Style, Media, Audio/Voice, 3D Asset, and Animation Clip selected revisions with explicit dependency impact.
7. Semantic capability discovery, concrete model `list/get`, workflow `list/get`, asset/upload/history search, and budget/cost discovery are distinct first-party contracts rather than one ambiguous catalog.
8. Model selection supports both automatic policy and explicit compatible user override; exact selected model/workflow/compiler versions are recorded in job lineage and silent substitution is forbidden.
9. Creative jobs support submit/get/wait/list/cancel, cross-turn retrieval, bounded retries/concurrency/results, owner/project isolation, current-turn media delivery, and durable Asset outputs.
10. Cost/compute preflight plus enforceable job/batch/session/project hard limits can block expensive work before execution; paid-adapter secrets/quota data remain adapter-owned and redacted.
11. Conversation attachments, external-client device uploads, reviewed web-URL imports, prior generations, and promoted Elements can all become reusable stable Assets through reviewed ingest paths.
12. External-client upload handoff is OAuth/owner/project bound, expiring, bounded, and replay-safe; URL import obeys shared SSRF/redirect/DNS/content-type/size policy; arbitrary local host paths are never the normal MCP upload mechanism.
13. Generation/upload history is queryable by typed filters/source tags and prior media can be reused directly by Asset ID without download/re-upload round trips.
14. Finished media can be reviewed in the current MCP/client turn through a bounded media/resource result while the same output persists as a queryable Asset.
15. MCP-core image utility parity is implemented through normal model/workflow/job/lineage semantics: reference generation/editing plus image upscale, background removal, and outpaint.
16. MCP-core video utility parity is implemented through normal model/workflow/job/lineage semantics: video generation/reference or image-to-video where active, reframe, upscale, background removal, motion control, and bounded clip extraction.
17. MCP-core audio parity is implemented through normal Asset/Element lineage: speech/voice, authorized voice cloning, voice change, and video dubbing; music/SFX remain supported semantic capabilities when an active adapter provides them.
18. Every utility/edit transform creates a child revision/Asset and never mutates an accepted parent in place.
19. Creative Graph execution supports typed validation, branching, parallel independent nodes, partial rerun, unaffected-output reuse, templates, node status/QA, and underlying approval/effect preservation.
20. A Canvas-style visual workspace edits/runs that same graph contract without moving relay credentials, Blender host authority, provider secrets, or unrestricted executable node payloads into the browser.
21. SceneBoard supports Auto and Manual planning with connected frames, reusable Elements, frame-level revision, continuity constraints, and hero-frame promotion.
22. Scene Director owns engine-neutral global scene look/lighting/palette plus per-shot framing/camera/lens/focal/aperture/movement/tempo state; Director suggestions never silently trigger expensive generation.
23. One fresh **S3 Scene Studio** benchmark produces a coherent 10–30 second cinematic scene with inspectable Scene/Shot/Element/job/QA state through generated-video, Blender, or a deliberate mixed backend.
24. Character identity supports a fictional-character path with structured visual/narrative fields independent of training; optional real-person identity adapters require explicit intent and never replace Character Element authority.
25. Blender is implemented as the retained 11-tool first-class optional capability with loopback-only bridge and truthful host-authority semantics.
26. One anime character reaches production-ready-enough mesh/UV/material/hair/clothing state for the chosen benchmark, receives a reusable body rig plus initial facial controls, and passes representative deformation QA.
27. A 3–5 second anime character performance passes structural and temporal review.
28. One fresh **A6 Anime Studio** benchmark produces a 10–30 second anime sequence with SceneBoard/Director state, continuity, Blender-editable assets where selected, visual/temporal/render/export QA, and reusable Elements/assets.
29. Game Studio freezes Game Design/Build Manifest + STYLE FORMULA + Asset Manifest before broad asset/code work, supports parallel generated assets plus editable source, and preserves stable runtime paths/lineage.
30. One fresh **G3 Game Studio** benchmark passes a complete browser-game loop including start/core loop/win-lose/restart, declared inputs, missing-asset/runtime-console checks, responsive rendering, and timing/performance assertions appropriate to the game.
31. When multiplayer is claimed, the selected local/online path passes reviewed **G4** acceptance; online mode proves at least two-session room/state behavior and cross-room/owner isolation.
32. One fresh **G5 Game Studio** deployment maps a shareable playable URL to an accepted source/build revision while source remains editable and public marketplace/catalog publication has not happened implicitly.
33. Follow-up game requests preserve existing source/Elements/assets when unaffected, amend manifests deliberately, rerun affected graph/build stages, and repeat playtest before acceptance.
34. Visual, temporal, structural, continuity, and gameplay QA distinguish hard fail, soft finding, and not-inspected; generation/build/deploy success is never treated as QA success by itself.
35. Surgical revision lineage can alter one accepted field/node/shot/asset/mechanic without overwriting unrelated locked state or provenance.
36. Generate/render/export/build, deploy/share, and public publish remain distinct lifecycle/effect boundaries.
37. A clean-room second-project falsification shows Scene, Anime, and Game contracts are not hard-coded to the first demo; at least one track reaches its repeatable-template milestone with materially different Elements/style/genre.
38. No arbitrary model-supplied endpoint/workflow graph/template, unrestricted Blender host/path, game networking credential, or secret-bearing generic terminal fallback bypasses reviewed capability boundaries.
39. Core MCP parity tests cover model/workflow discovery, safe upload/import, history/reuse, cost/budget preflight, image/video/audio utilities, job lifecycle, and media delivery in addition to Scene/Anime/Game production tests.
40. Extended MCP parity items—localization/subtitles/shorts, UGC/faceless/motion-design packaged workflows, identity edits, relight/weather/object edits, restore/stabilize/time-remap/color-match, and marketing/site verticals—have explicit shared-kernel owners and priorities; none requires a second state/job/asset platform.
41. Relevant focused tests and `pnpm guardrail:fast` / affected-stack full gates / `pnpm guardrail:full` pass before closure.
42. A generic external MCP client can complete the parity acceptance journey—discover models/workflows, upload/import, estimate, generate, receive media, transform, browse history, reuse an Asset/Element, run a multi-step skill/graph, and hit a budget denial—without Higgsfield, CLI, local shell, or arbitrary host paths.
43. Operator-only actions such as GPU/model installation, Blender/add-on setup, relay restart, production deployment credentials, or external public publishing are reported explicitly and are not performed implicitly by the implementation agent.

## Current execution state

As of 2026-09-13:

- `origin/main` is `e13bd38666ec4cb100ae713bd8272426db209d0e` at the planning baseline; implementation must re-audit current `main` again before source work;
- Plan 069 did not exist on that `origin/main` baseline; 069 was the next unused numeric plan there;
- the earlier Blender-only Plan 069 lived on the local `release/ai-tools-v0.0.15` branch and had no Blender source implementation started;
- the retained Blender design still owns its loopback bridge, security, 11-tool surface, 3D/anime workflow guidance, attachment ingress, QA, asset I/O, and checkpoint decisions;
- the first rewrite broadened the target to an anime-first Creative Production Platform, but this second review found that framing too narrow for the user's actual target;
- the top-level product target is now explicitly **one Creative Production Platform with first-class Scene Studio, Anime Studio, and Game Studio tracks** over one shared Creative Project / Element Library / Capability / Job / Creative Graph / QA kernel;
- public Higgsfield behavior was re-audited through its skills plus current Canvas, Popcorn, Cinema Studio, Elements, Soul Cast, and Games/Supercomputer documentation, and the plan now maps those product layers instead of comparing only MCP/CLI skills;
- a third parity pass on 2026-09-13 audited the official Higgsfield MCP landing page/help-center/Claude workflow specifically; it added explicit contracts for concrete model list/get and exact-model override, image/video upscale, image/video background removal, image outpaint, video reframe, motion control, voice clone/change/video dubbing, Personal-Clipper-style extraction, credit/cost preflight, generation/upload history, prior-Asset reuse, external-client upload handoff, safe URL import, current-turn media delivery plus durable Asset persistence, and full skill execution through the same MCP surface;
- Canvas-style graph composition, Popcorn-style Auto/Manual SceneBoard, Cinema-Studio-style Elements/Director/cinematography/Hero-Frame workflow, and Supercomputer/Game-Generation-style design/assets/build/playtest/multiplayer/deploy behavior are first-class roadmap scope;
- Higgsfield remains a public behavioral/product benchmark only, not a runtime dependency;
- game documentation currently shows an evolving command/project surface; the plan therefore freezes behavior contracts rather than copying transient Higgsfield command names;
- no production implementation files were changed as part of this planning review.
