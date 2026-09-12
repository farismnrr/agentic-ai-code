# Plan 069 — Anime-first Creative Production Platform

Status: **PLANNED — supersedes the Blender-only Plan 069 scope before implementation; no production source implementation has started**

Created: 2026-09-11
Updated: 2026-09-12

## Goal

Build an anime-first **Masih Awam Creative Production Platform** that can take a creative brief from reference intake through reusable character/world assets, generation, Blender production, animation, visual/temporal QA, compositing, and final delivery **without depending on Higgsfield accounts, Higgsfield MCP, Higgsfield CLI, Higgsfield APIs, or Higgsfield-hosted generation**.

The platform should learn from the strongest public operating patterns in Higgsfield's MIT-licensed skills — skill routing, minimal interviews, live capability discovery, reusable identity/style state, job orchestration, workflow chaining, concept gates, visual QA, surgical revisions, and delivery contracts — while owning the runtime, state, skills, engine adapters, Blender integration, and production methodology inside Masih Awam.

The first quality target is deliberately narrower than “make a whole anime episode”: **produce one short, coherent anime/stylized sequence with a reusable character, stable visual identity, a real Blender scene/rig/animation path, temporal review, and reproducible project artifacts.** Expand only after that benchmark is credible.

## Success criteria

Plan 069 is successful when Masih Awam can eventually demonstrate all of the following through its own first-party contracts:

1. A user can give a text brief plus image references and receive a bounded production interview rather than an immediate uncontrolled generation call.
2. The project freezes reusable creative state before expensive production: character identity, visual style, world/location direction, asset manifest, shot plan, and provenance.
3. Skills describe **how to produce** anime assets/shots while tools describe **what the runtime may do**; skills do not hard-code one provider/model as product architecture.
4. Runtime capability discovery can report available image/video/audio/3D/DCC capabilities and their schemas before a workflow commits to them.
5. Generation jobs have first-party create/get/wait/cancel/result semantics, deterministic project ownership, bounded outputs, and retained source metadata.
6. Reusable identity/style state can drive multiple generations without relying on a Higgsfield Soul ID or other Higgsfield identifier.
7. A user/uploaded or generated reference can cross a reviewed attachment-to-workspace boundary and become a stable production asset.
8. Blender remains a first-class persistent DCC backend for modeling, retopology, UVs, materials, hair/clothing, rigging, facial setup, animation, cameras, lighting, rendering, compositing, import/export, and checkpoints.
9. The system can inspect and visually review outputs, reject failed results, and perform narrowly scoped revisions instead of blindly regenerating everything.
10. The first end-to-end benchmark produces a reusable anime character and a **10–30 second single-scene/small multi-shot sequence** with inspectable project state, temporal QA, and contained final outputs.
11. Higgsfield is used only as a public design/reference benchmark during planning; no shipping code requires Higgsfield auth, tokens, binaries, endpoints, model IDs, or services.
12. Relevant repository guardrails pass and the plan remains truthful about any engine/model capability that is not yet implemented or locally available.

## Scope

### In scope

- first-party creative skill architecture and reference/guidance conventions;
- anime production methodology and staged workflow gates;
- creative project state and asset/provenance manifests;
- capability discovery separate from skill logic;
- provider/engine-neutral generation contracts;
- local/operator-owned generation-engine adapters, with an initial implementation chosen only after a fresh implementation-time audit;
- first-party generation job lifecycle and reusable outputs;
- identity/style/world state suitable for fictional anime characters, not only real-person face identity;
- safe attachment/materialization into production workspaces;
- Blender as a first-class live-session DCC backend;
- 2D reference/turnaround -> 3D asset -> rig -> animation -> shot -> render workflows;
- visual and temporal QA loops;
- audio/voice/music capability routing needed by anime production;
- deterministic assembly/compositing/export where practical;
- skill evaluations and production acceptance fixtures;
- incremental milestones from still-image consistency to a short anime sequence.

### Out of scope for the first implementation

- cloning or reverse-engineering Higgsfield private backend behavior;
- requiring Higgsfield MCP/CLI/auth/API as a runtime dependency;
- promising tool/model parity by raw model count;
- one-click full-length episode generation before shorter benchmarks pass;
- autonomous publication to public marketplaces/social media without explicit user approval;
- a generic unrestricted plugin system that can execute arbitrary model-supplied workflow code;
- arbitrary remote Blender hosts in v1;
- hundreds of atomic Blender MCP tools when a smaller inspected execution surface is sufficient;
- treating generated hidden views of a character as factual ground truth when only one reference view exists;
- silently training on copyrighted/private references beyond the user's authorized production inputs;
- making one visual style (“anime”) synonymous with one hard-coded shader, model, prompt, or topology recipe.

## Core product decision

**Blender is not the Higgsfield replacement.** Blender is a production/DCC engine. The new Plan 069 owns the higher-level Creative OS that sits above Blender and any generative engines.

```text
User / Agent
    |
    v
Masih Awam Creative Skills
    |  intent routing, interview gates, production methodology
    v
Creative Project State
    |  character/style/world/asset/shot/audio/provenance manifests
    v
Capability + Workflow Router
    |
    +-------------------+-------------------+------------------+
    |                   |                   |                  |
    v                   v                   v                  v
Image/Video Engine   Audio Engine        3D Bootstrap      Blender DCC
    |                   |                   |                  |
    +-------------------+-------------------+------------------+
                                |
                                v
                      Visual / Temporal QA
                                |
                                v
                    Composite / Export / Delivery
```

The important architectural rule is:

> **Skills request capabilities; adapters choose engines.**

An anime skill should ask for semantics such as `image.reference_generate`, `video.image_to_video`, `audio.voice`, `3d.image_to_mesh`, or `dcc.character_rig`. It must not make the product architecture depend on a transient vendor/model identifier.

## External benchmark audited on 2026-09-12

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

Current public repository inventory exposes **nine skills**: generate, soul-id, product-photoshoot, brandkit, marketplace-cards, websites, video-explainer, youtube-thumbnail, and game-generation. Some repository guidance text still uses an older “seven skills” description; implementation-time audits must prefer the live repository inventory/README rather than copying stale counts.

The benchmark is used for interoperability/design learning only. The shipping architecture below is independent.

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

| Higgsfield behavior | Why it matters | Masih Awam equivalent | Anime-first use |
| --- | --- | --- | --- |
| Skill auto-trigger and `Use when` / `NOT for` boundaries | avoids one giant ambiguous agent prompt | first-party creative skills with explicit trigger/route-out contracts | route character design vs shot production vs rigging vs key art correctly |
| Small decision-oriented `SKILL.md`, large on-demand `references/` | controls context cost | keep routing/stage logic compact; load detailed anime references only after a path is chosen | anatomy/proportion, topology, toon shading, facial rig, animation principles loaded only when needed |
| Minimal, mode-specific interviews | gathers only information that changes production | typed intake gates per skill | ask style/character/shot questions once; do not interrogate the user repeatedly |
| Live `model list/get` | prevents stale model assumptions | `creative_capabilities` / adapter schema discovery | know which reference-image, video, audio, 3D, or resolution features are actually available |
| Separate `workflow list/get` from model catalog | treats chains as first-class products | first-party workflow registry separate from engine catalog | turnaround generation, image->3D bootstrap, rigging, lipsync, shot render are workflows, not “models” |
| Media role validation | avoids malformed generation inputs | typed creative asset roles and adapter validation | `character_front`, `character_side`, `style_reference`, `motion_reference`, `voice_reference`, etc. |
| Auto-upload local path / reuse previous job output | lets outputs chain naturally | reviewed workspace materialization + stable asset IDs | concept art -> turnaround -> mesh bootstrap -> Blender reference -> shot |
| Soul ID reusable identity | consistency survives many generations | Character Identity Pack | recurring anime character appearance, proportions, costume, face/hair, palette, later rig/voice bindings |
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
| Skill chaining via returned IDs | keeps boundaries explicit | typed asset/project references | Character Pack -> Turnaround -> Blender Asset -> Rig -> Shot -> Render |
| Game `STYLE FORMULA` + asset manifest | global coherence before parallel asset work | Style Bible + Asset Manifest | every anime character/prop/environment asset declares style and production role before generation |
| Website create -> repo edit -> deploy | creation and publication are separate lifecycles | build/export and publish remain distinct | rendering/export never silently uploads/publishes finished anime |
| Eval scenarios/version sync | skills remain testable products | creative skill eval suite + versioned contracts | prove routing, state preservation, QA, and anime benchmark behavior over time |

## 1:1 mapping of the nine Higgsfield skills

This is a **behavioral mapping**, not a plan to copy their product names or vendor-specific prompts.

| Higgsfield skill | Core mechanic to learn | Masih Awam native counterpart | Anime priority |
| --- | --- | --- | --- |
| `higgsfield-generate` | broad media router + live schema discovery + jobs + reusable media | `creative-generate` capability/skill | **P0** foundation |
| `higgsfield-soul-id` | one-time reusable identity training/reference | `character-identity` / Character Identity Pack | **P0**; generalized to fictional characters, not only faces |
| `higgsfield-brandkit` | lock palette/type/logo/style, dependency-aware revisions | `style-bible` + World/Visual Bible | **P0**; becomes the anime visual-system authority |
| `higgsfield-product-photoshoot` | mode router + short interview + specialist prompt enhancer | `anime-key-art` / `anime-stills` | **P1**; portraits, action stills, expression sheets, environment/key art |
| `higgsfield-marketplace-cards` | fixed deliverable bundle from one product identity | `anime-promo-pack` | **P3**; character cards, poster/key visual, episode/social promo assets |
| `higgsfield-video-explainer` | style lock + block planning + audio-first dependency + assembly | `anime-sequence` / storyboard->animatic->shot DAG | **P1/P2**; replaces fixed 10-second explainer blocks with shot-aware timing |
| `higgsfield-youtube-thumbnail` | truthful concept gate + variants + visual QA + surgical edits | `anime-key-visual` / thumbnail skill | **P2**; useful for episode cover/poster and validates QA patterns |
| `higgsfield-game-generation` | design manifest + STYLE FORMULA + asset CSV + build/QA/deploy | `anime-production` master skill; later interactive/game spinoff | **P0 methodology**, **P4** game parity |
| `higgsfield-websites` | scaffold/edit/test/deploy lifecycle and media chaining | existing Masih Awam coding agent + `anime-project-site` guidance | **P4**; not on anime core critical path |

The anime MVP does **not** need to implement all nine user-facing skill categories before it can ship value. It must, however, implement the underlying mechanics that make those skills effective: routing, state, manifests, jobs, references, QA, revisions, and chaining.

## Anime-first creative state

Higgsfield's reusable state is split across Soul IDs, products, brand kits, presets, job IDs, and website/game resources. Masih Awam should use a more general project model.

### Creative Project Manifest

One project-level manifest identifies the authoritative production state and references all sub-manifests. It must be human-readable, diffable, versionable, and free of secrets.

Initial logical fields:

- project identity/title and production intent;
- canonical user brief and approved assumptions;
- target format/aspect/FPS/resolution where known;
- selected style pack;
- character packs;
- world/location packs;
- asset manifest;
- shot/sequence plan;
- audio/voice plan;
- generation/revision lineage references;
- source/provenance records;
- export/delivery targets;
- production status and QA findings.

Do not turn this into a database-first subsystem prematurely. Begin with workspace-contained structured files and promote to product persistence only when multi-session/product UX requires it.

### Character Identity Pack

Generalize Higgsfield Soul ID into a reusable fictional-character contract:

- authoritative reference images and their roles;
- name/ID and design notes;
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

### Shot / Sequence Manifest

Anime is temporal. A shot manifest must become first-class before long-form generation:

- shot ID and sequence order;
- duration/frame range/FPS;
- characters/assets/location;
- staging/blocking;
- camera/lens/movement;
- action and performance beats;
- dialogue/audio cue references;
- expression/pose requirements;
- FX/lighting notes;
- required upstream assets;
- current storyboard/animatic/Blender scene/action/render references;
- continuity constraints;
- QA state.

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

### Gate B — Style and identity lock

Do not start expensive multi-asset work until:

- Style Pack has an approved direction;
- each recurring main character has a Character Pack with at least usable canonical references;
- contradictions between references are recorded/resolved;
- generated missing turnaround views are labeled as interpretations.

### Gate C — Asset manifest

Before parallel asset generation:

- list required character/prop/environment/audio assets;
- declare dependency relationships;
- define acceptable output/QA criteria per asset;
- choose which assets need 2D-only, 3D-only, or both.

### Gate D — Shot plan

Before animation/render batches:

- freeze sequence order and representative shot intent;
- identify dialogue/timing dependencies;
- verify required assets/rigs exist;
- checkpoint the Blender scene before destructive scene-wide work.

### Gate E — Visual/temporal review

Never claim production completion from tool exit status alone. Inspect representative frames/images and structural state. Failed identity/style/deformation/readability checks trigger bounded revision.

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
image.generate
image.reference_generate
image.edit
image.inpaint
image.control_pose
image.control_depth
image.upscale

video.generate
video.image_to_video
video.reference_generate
video.extend
video.reframe
video.lipsync

3d.image_to_mesh
3d.text_to_mesh
3d.texture
3d.rig_bootstrap

audio.voice
audio.speech
audio.music
audio.sfx
audio.voice_conversion

dcc.inspect
dcc.execute
dcc.import
dcc.export
dcc.preview
dcc.render
dcc.checkpoint
```

This vocabulary is a planning target, not a requirement to expose one MCP tool per line. The implementation should keep the public surface compact and allow one tool to advertise multiple semantic capabilities through discovery.

## Generation-engine strategy

Do not recreate Higgsfield by replacing it with another hard-coded cloud vendor.

### V1 decision rule

At implementation time, select the smallest operator-owned engine substrate that can satisfy the first anime milestones. **ComfyUI is the preferred candidate to re-audit** because it can act as a local graph/runtime for multiple image/video/3D workflows, but this plan does not claim it is installed or freeze it without a fresh compatibility/security review.

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

Model selection belongs to runtime policy based on:

- required capability;
- reference/identity support;
- resolution/duration constraints;
- latency/resource budget;
- local hardware availability;
- user preference;
- measured quality on anime evals;
- license/usage constraints.

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

Do not skip levels merely because a model can output a flashy video.

| Milestone | Deliverable | Must prove before advancing |
| --- | --- | --- |
| M0 | Creative project + manifests only | skill routing, project/asset/shot state are coherent and versionable |
| M1 | Consistent anime still set | one character remains recognizably consistent across controlled views/expressions |
| M2 | Accepted turnaround + Style Pack | authoritative/interpreted references are distinguished; asset manifest is usable |
| M3 | Blender character asset | contained import, cleanup/retopo/UV/material inspection, multi-angle visual review |
| M4 | Reusable rigged character | skeleton, weights, facial controls, representative deformation QA |
| M5 | 3–5 second animation | structural animation inspection + temporal preview + render review |
| M6 | 10–30 second anime scene | shot manifest, character continuity, camera/lighting, audio timing as applicable, final QA |
| M7 | 30–60 second multi-shot short | cross-shot continuity, reusable assets, render/composite pipeline |
| M8 | repeatable short-production template | second project/character can reuse the platform without hard-coded first-demo assumptions |

A full episode or autonomous series pipeline is explicitly **post-M8**.

## Skill architecture

The final names should be frozen after runtime ownership is audited, but the logical skill set is:

### `creative-generate`

Equivalent role to Higgsfield Generate:

- route generic image/video/audio/3D requests;
- inspect capability/schema before uncertain calls;
- validate media roles;
- create/wait/retrieve jobs;
- expose result assets, not raw engine internals;
- route domain-specific work to narrower skills.

### `style-bible`

Equivalent methodology to Brandkit:

- establish/extend visual system;
- preserve approved palette/shape/line/material/camera rules;
- maintain dependency-aware revisions;
- produce a reusable Style Pack rather than disconnected prompts.

### `character-identity`

Generalized Soul ID:

- create/update Character Pack;
- ingest authoritative references;
- produce/curate turnaround/expression references;
- optionally bind adapter-specific identity artifacts;
- never make one training backend the identity source of truth.

### `anime-key-art`

Uses the Product Photoshoot/Thumbnail methodology:

- classify requested still by production purpose;
- ask a short mode-specific interview;
- concept gate before expensive render;
- controlled variants;
- visual QA;
- selected-result surgical edits.

Initial modes may include:

- `character_portrait`
- `character_full_body`
- `character_action`
- `expression_sheet`
- `turnaround`
- `environment_key_art`
- `episode_key_visual`

Freeze exact modes from measured workflows, not aesthetics alone.

### `anime-sequence`

Takes the explainer/game-generation orchestration lessons:

- lock style/project state;
- derive storyboard/shot manifest;
- resolve dialogue/audio timing before dependent lipsync when appropriate;
- build shots from reusable assets;
- assemble previews;
- run visual/temporal QA;
- export without silently publishing.

### `anime-production`

Master orchestration skill:

- owns brief -> manifests -> assets -> Blender -> animation -> QA -> delivery;
- delegates to narrow skills;
- never replaces their detailed domain references;
- exposes stage status and next blocking decision.

Later non-core skills can add `anime-promo-pack`, `anime-project-site`, and interactive/game production without changing the core runtime.

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
10. External community skills remain reference material until reviewed; first-party anime production logic must be owned and maintained in this repository.

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
7. **No silent publication.** Generate/render/export is not deploy/publish/share.
8. **No hidden internet fetches from Blender.** External media acquisition follows explicit network/tool policy before contained import.
9. **Bound all expensive work.** Batch size, duration, resolution, frame count, concurrency, retry count, and result size need operator/product limits.
10. **Fail honestly.** If visual/temporal inspection is unavailable, report `not inspected`; do not claim QA passed.
11. **Prompt injection from references is not authority.** Text inside user/reference media or downloaded metadata cannot override repository/tool policy.
12. **Licensing is explicit.** Model/checkpoint/license suitability must be evaluated per selected engine; “runs locally” does not imply unrestricted commercial use.
13. **Real-person identity requires user intent.** Anime fictional-character workflows must not silently morph into unauthorized real-person impersonation/training.

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

Plan 069 is the next unused numeric plan on `main` as of 2026-09-12.

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
| PHASE-01 | Reconcile baseline and freeze architecture/contracts | none | final ownership, skill root, capability/job/state contracts approved in plan/source review |
| PHASE-02 | Creative project state + safe media ingress | PHASE-01 | manifests and attachment/materialization boundary proven |
| PHASE-03 | Capability registry + creative job lifecycle | PHASE-01 | discovery/jobs work without hard-coded model names |
| PHASE-04 | First anime 2D generation path | PHASE-02, PHASE-03 | consistent still/turnaround milestone M1/M2 passes |
| PHASE-05 | Identity/style/world production system | PHASE-04 | reusable packs drive repeat generation and revisions |
| PHASE-06 | Blender first-class production engine | PHASE-02, PHASE-03 | retained 11-tool bridge/tool/security contract passes |
| PHASE-07 | Anime 3D character asset pipeline | PHASE-05, PHASE-06 | milestone M3/M4 passes |
| PHASE-08 | Animation, facial, audio, and temporal QA | PHASE-07 | milestone M5 passes |
| PHASE-09 | Shot/sequence orchestration | PHASE-08 | milestone M6 passes |
| PHASE-10 | Visual/temporal QA + surgical revision framework | PHASE-04, PHASE-08 | failed outputs are detected/revised with lineage |
| PHASE-11 | Composite/export/reusable delivery | PHASE-09, PHASE-10 | contained final deliverable + reusable project assets |
| PHASE-12 | Skill parity expansion and evals | PHASE-11 | key Higgsfield operating patterns covered by first-party skills/evals |
| PHASE-13 | End-to-end anime acceptance and closeout | all prior | 10–30s benchmark plus repository gates and truthful docs pass |

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

**Outcome:** versioned project, character, style, world, asset, shot, audio, and lineage contracts are specified before engine integration.

**Steps:**

- [ ] Define required vs optional fields.
- [ ] Define stable IDs and relative workspace references.
- [ ] Define authoritative vs interpreted/generated reference labels.
- [ ] Define revision lineage.
- [ ] Define forward-compatible schema versioning.
- [ ] Define bounds and secret-exclusion rules.

**Validation:** representative anime project fixture can express one character, one location, assets, three shots, audio cues, and revision lineage without engine-specific IDs as its source of truth.

**Commit boundary:** `feat(creative): add production manifest contracts`.

### TASK-004 — Freeze capability and job contracts

**Outcome:** semantic capabilities, adapter discovery, job lifecycle, effect policy, and result assets are specified independently from engines.

**Steps:**

- [ ] Freeze semantic capability vocabulary needed through M6.
- [ ] Freeze `capability list/get` representation.
- [ ] Freeze workflow registry separate from engine/model inventory.
- [ ] Freeze job submit/get/wait/cancel behavior.
- [ ] Freeze result and failure bounds.
- [ ] Freeze external-cost/network vs local-compute effect classifications.
- [ ] Decide whether existing relay Tasks can own creative long-running jobs directly or need a thin creative domain layer over the same manager.

**Validation:** mock adapters can advertise different engines for the same semantic capability without changing skill contracts.

**Commit boundary:** `feat(creative): define capability and job contracts`.

**Phase exit criteria:**

- [ ] no implementation depends on a Higgsfield contract;
- [ ] one skill root is authoritative;
- [ ] creative state schemas are frozen for initial milestones;
- [ ] adapter/job semantics reuse existing platform primitives where possible;
- [ ] Blender remains a separate privileged DCC capability under the master architecture.

# PHASE-02 — Creative project state and safe media ingress

**Goal:** make references and generated outputs durable, contained production assets.

**Dependencies:** PHASE-01.

### TASK-005 — Implement contained creative project workspace layout

**Outcome:** each project has bounded manifests and asset/revision directories without escaping authorized workspaces.

**Steps:**

- [ ] Define contained path layout.
- [ ] Ensure paths remain relative/canonical and cannot target protected credentials.
- [ ] Add atomic manifest updates.
- [ ] Preserve human-readable diffs.
- [ ] Bound manifest and metadata sizes.

**Validation:** traversal/symlink/protected-path tests fail closed; normal project fixture round-trips.

**Commit boundary:** `feat(creative): add contained project state`.

### TASK-006 — Implement attachment-to-workspace materialization

**Outcome:** user uploads and product-generated images/audio/video can become safe stable workspace files when production tools need bytes.

**Steps:**

- [ ] Reuse an existing reviewed first-party handoff if present.
- [ ] Otherwise add the smallest owning-layer materialization primitive.
- [ ] Validate media type, size, destination ownership, filename/path, and source metadata.
- [ ] Never accept arbitrary host destination paths.
- [ ] Preserve a source/provenance record.
- [ ] Keep model-visible attachment access distinct from Blender/engine-readable workspace files.

**Validation:** valid fixture materializes; oversized/unsupported/path-injection cases fail; Blender/engine can consume only the contained result.

**Commit boundary:** `feat(workspace): materialize creative attachments safely`.

### TASK-007 — Implement asset registry and revision lineage

**Outcome:** imported/generated/revised assets get stable project IDs and parent/child lineage.

**Steps:**

- [ ] Register source type and role.
- [ ] Record output metadata/checksum where appropriate.
- [ ] Link generation job and parent revision.
- [ ] Mark accepted vs candidate revisions.
- [ ] Keep raw credentials/prompts out of routine metadata.

**Validation:** fixture can trace user reference -> generated turnaround -> revised selected image -> Blender import.

**Commit boundary:** `feat(creative): track asset revision lineage`.

**Phase exit criteria:**

- [ ] production media can be safely materialized;
- [ ] every asset has bounded provenance/lineage;
- [ ] no Blender/generator path assumes transient chat paths are filesystem paths.

# PHASE-03 — Capability registry and creative jobs

**Goal:** reproduce Higgsfield's live discovery/job strengths without vendor coupling.

**Dependencies:** PHASE-01.

### TASK-008 — Add creative capability discovery

**Outcome:** clients/skills can ask what semantic image/video/audio/3D capabilities are active and inspect their parameter bounds.

**Steps:**

- [ ] Add operator-disabled-by-default creative capability group.
- [ ] Expose active semantic capabilities and workflow IDs.
- [ ] Keep adapter/model implementation metadata bounded.
- [ ] Return activation/setup hint when disabled.
- [ ] Keep discovery separate from general model routing in skill docs.

**Validation:** mock capability set appears/disappears with operator config and schema remains stable when adapter names change.

**Commit boundary:** `feat(creative): expose capability discovery`.

### TASK-009 — Add first-party creative job lifecycle

**Outcome:** generation work can be started, polled, waited, cancelled, and retrieved with stable project ownership.

**Steps:**

- [ ] Reuse existing job/task manager lifecycle and cancellation where possible.
- [ ] Add domain metadata only where creative workflows need it.
- [ ] Bound concurrent jobs/batches.
- [ ] Persist/recover only if current platform task semantics cannot satisfy cross-turn retrieval safely.
- [ ] Keep failure diagnostics redacted/classified.

**Validation:** fake adapter covers queued/running/completed/failed/cancelled, lost-client polling, timeout, output bounds, and owner isolation.

**Commit boundary:** `feat(creative): add generation job lifecycle`.

### TASK-010 — Add curated workflow registry

**Outcome:** multi-step workflows are discoverable independently from individual engine/model capabilities.

Initial candidate workflows to freeze from actual engine support:

- `character_turnaround`
- `expression_sheet`
- `pose_sheet`
- `image_to_3d_bootstrap`
- `character_rig_bootstrap`
- `image_to_video_preview`
- `lipsync_preview`
- `shot_render_preview`

**Validation:** workflows advertise required inputs/output roles and reject unsupported parameters before engine execution.

**Commit boundary:** `feat(creative): add workflow registry`.

**Phase exit criteria:**

- [ ] live capability discovery works;
- [ ] workflows are distinct from engine/model catalog;
- [ ] job lifecycle is bounded and owner/project scoped;
- [ ] skills can make routing decisions without hard-coded provider IDs.

# PHASE-04 — First anime 2D generation path

**Goal:** reach M1/M2 before solving full 3D production.

**Dependencies:** PHASE-02, PHASE-03.

### TASK-011 — Select and secure the initial generation adapter

**Outcome:** one reviewed adapter can perform reference-aware image generation/editing for anime fixtures.

**Steps:**

- [ ] Re-audit ComfyUI as preferred candidate and compare against direct/local alternatives.
- [ ] Confirm operator setup, endpoint/loopback policy, executable/custom-node trust model, result format, cancellation, and model/license constraints.
- [ ] Choose the smallest adapter that satisfies M1/M2.
- [ ] Keep arbitrary model-supplied workflow graphs disabled in v1.
- [ ] Add operator config/capability activation docs.

**Validation:** safe mock/fixture tests plus operator-only manual smoke; no Higgsfield dependency.

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

**Validation:** M1/M2 fixture shows recognizable identity/style consistency across the selected set and records QA findings honestly.

**Commit boundary:** `feat(anime): build character reference workflow`.

**Phase exit criteria:**

- [ ] one character can be reproduced across controlled stills;
- [ ] style and character state are reusable;
- [ ] visual QA can reject/promote revisions;
- [ ] accepted turnaround is ready for 3D production.

# PHASE-05 — Identity, style, and world production system

**Goal:** turn the first still workflow into reusable production state rather than demo prompts.

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

**Goal:** reach M3/M4 using the accepted Character/Style Packs and Blender engine.

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

**Validation:** M4 deformation/facial fixture passes structural + visual review.

**Commit boundary:** `feat(anime): rig reusable character`.

**Phase exit criteria:**

- [ ] one project character is reusable in a fresh scene;
- [ ] topology/UV/material/rig/facial state is inspectable;
- [ ] representative deformation is visually accepted;
- [ ] asset can be exported and re-imported within contained workspace authority.

# PHASE-08 — Animation, facial, audio, and temporal QA

**Goal:** reach M5 with real motion, not only rig existence.

**Dependencies:** PHASE-07.

### TASK-026 — Add initial audio/voice capability adapter

**Outcome:** a reviewed local/operator-owned or explicitly configured adapter can create/import dialogue timing needed for the benchmark.

**Steps:**

- [ ] Freeze voice/audio semantic contract.
- [ ] Preserve voice/source/license provenance.
- [ ] Keep credentials isolated if a non-local provider is later supported.
- [ ] Produce contained audio asset and timing metadata.

**Validation:** fixture generates/imports bounded speech and preserves project ownership.

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

# PHASE-09 — Shot and sequence orchestration

**Goal:** compose reusable assets into M6, a 10–30 second anime scene.

**Dependencies:** PHASE-08.

### TASK-030 — Implement storyboard/shot-manifest skill

**Outcome:** an approved brief becomes a bounded shot list before render-heavy work starts.

**Steps:**

- [ ] Generate/author storyboard candidates cheaply.
- [ ] Select camera/action/dialogue beats.
- [ ] Freeze shot IDs/durations/assets/continuity.
- [ ] Validate required upstream assets exist.
- [ ] Keep user approval/autonomy behavior explicit.

**Validation:** shot manifest is sufficient to reproduce blocking without hidden chat context.

**Commit boundary:** `feat(anime): add shot planning workflow`.

### TASK-031 — Implement blocking -> animation -> render-preview DAG

**Outcome:** each shot progresses through explicit states and can fail/retry independently.

**Validation:** one failed shot does not invalidate unrelated accepted shots; dependencies and job states are visible.

**Commit boundary:** `feat(anime): orchestrate shot production`.

### TASK-032 — Implement continuity checks

**Outcome:** cross-shot character, costume, prop, location, direction, lighting, and action continuity is reviewed before final render.

**Validation:** seeded continuity fixture produces a detectable finding and can be resolved with a scoped revision.

**Commit boundary:** `feat(anime): validate shot continuity`.

**Phase exit criteria:**

- [ ] 10–30 second sequence has bounded shot state;
- [ ] reusable character/world assets survive shot changes;
- [ ] blocking/animation/render preview can be inspected and revised shot by shot.

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

# PHASE-11 — Composite, export, and reusable delivery

**Goal:** turn accepted shots into contained deliverables without hidden publication.

**Dependencies:** PHASE-09, PHASE-10.

### TASK-037 — Implement deterministic sequence assembly

**Outcome:** accepted shot renders/audio can be assembled with explicit ordering, frame rate, resolution, and audio sync.

**Validation:** deterministic fixture assembles the same bounded sequence from manifest state.

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
- [ ] no publish/deploy action is hidden in export;
- [ ] a later session can continue from project state.

# PHASE-12 — Skill parity expansion and evals

**Goal:** turn the successful anime pipeline into a maintainable creative skill system comparable in operating quality to Higgsfield's public skill suite.

**Dependencies:** PHASE-11.

### TASK-040 — Add first-party creative skill eval scenarios

**Outcome:** routing and workflow decisions are testable rather than preference-driven.

Eval categories:

- direct trigger;
- natural paraphrase;
- nearby request that should route out;
- missing-input interview behavior;
- user-autonomy delegation;
- capability unavailable behavior;
- identity/style preservation;
- asset chaining;
- QA hard-failure handling;
- surgical edit behavior;
- no silent publish behavior.

**Validation:** eval runner produces deterministic routing/contract results without requiring expensive generation for every case.

**Commit boundary:** `test(skills): cover creative routing and gates`.

### TASK-041 — Add `anime-promo-pack` methodology

**Outcome:** reproduce the fixed-bundle strength of marketplace cards for anime deliverables: character card, episode key visual, landscape/vertical cover, and optional social crop from one approved project state.

**Validation:** all outputs derive from accepted Character/Style Packs and preserve factual project content.

**Commit boundary:** `feat(anime): add promo pack skill`.

### TASK-042 — Add project-site integration only after core production is stable

**Outcome:** existing coding capabilities can build a project/portfolio site from contained accepted media without a new Higgsfield-like hosted website dependency.

**Validation:** site build is separate from deploy/publish and uses normal repository coding/deployment policy.

**Commit boundary:** `feat(anime): add project site workflow`.

### TASK-043 — Evaluate interactive/game spinoff reuse

**Outcome:** verify whether the same Character/Style/Asset Packs can feed existing/future game-production workflows; do not duplicate a second asset identity system.

**Validation:** design review demonstrates shared asset contracts or records why an adapter is required.

**Commit boundary:** docs/skill change only unless interactive runtime work is separately authorized.

**Phase exit criteria:**

- [ ] creative skills have testable routing contracts;
- [ ] additional deliverables reuse core project state;
- [ ] no marketing/game/site feature blocks the anime core path.

# PHASE-13 — End-to-end anime acceptance and closeout

**Goal:** prove the platform as a production system, not a collection of tools.

**Dependencies:** all prior phases.

### TASK-044 — Run M6 anime benchmark

**Outcome:** one 10–30 second sequence is produced from a fresh project through the intended first-party path.

Acceptance journey:

1. user brief + reference intake;
2. Style Pack approval;
3. Character Pack/turnaround creation and QA;
4. asset manifest freeze;
5. contained reference materialization;
6. Blender character bootstrap;
7. mesh/UV/material/hair/clothing production;
8. rig/weights/facial controls;
9. storyboard/shot manifest;
10. body/facial/audio timing;
11. temporal review and scoped corrections;
12. camera/lighting/toon render/compositor;
13. final assembly/audio sync;
14. contained export and reusable character asset;
15. project handoff bundle.

**Validation:** every stage leaves inspectable project/asset/job/QA evidence; no hidden Higgsfield dependency; no manual undocumented file teleportation between stages.

**Commit boundary:** no benchmark output should be committed unless it is a deliberately small repository fixture; normal generated production artifacts remain outside source history.

### TASK-045 — Run clean-room second-project falsification

**Outcome:** prove the architecture is not hard-coded to the first demo character/style.

**Validation:** a materially different anime character/style can reach at least M2/M3 using the same contracts without source-code changes to encode that character.

### TASK-046 — Security and failure matrix

**Outcome:** re-test path escapes, protected credentials, engine endpoint policy, arbitrary workflow/code injection, job ownership, cancellation, output bounds, Blender host authority, malicious media metadata, and publication separation.

**Validation:** relevant focused Rust/Nuxt tests plus security review pass.

### TASK-047 — Repository closure

**Steps:**

- [ ] Run focused subsystem tests while iterating.
- [ ] Run `pnpm guardrail:fast` before checkpoint commits.
- [ ] Run affected full Rust/Nuxt gates as required by changed ownership.
- [ ] Run `pnpm guardrail:full` before closure.
- [ ] Run dependency/security audits when dependency changes justify them.
- [ ] Update operator docs, architecture/security docs, skills/resources, canonical memory, and this plan's status/checklists truthfully.
- [ ] Review `.agents/knowledge/self-improvement.md`.
- [ ] Deliver through short-lived branch -> PR -> reviewed merge to `main`; do not bypass hooks or self-merge without authorization.
- [ ] Keep relay restart/deployment/operator GPU/model installation as explicit external actions, not implied by source completion.

**Phase exit criteria:**

- [ ] M6 benchmark passes;
- [ ] second-project falsification shows generality;
- [ ] security/failure matrix passes;
- [ ] docs and runtime contracts agree;
- [ ] no Higgsfield runtime dependency exists;
- [ ] repository closure gates pass.

## Test strategy

Use repository-native test locations only. Do not add `verify-069` or other plan-numbered scripts.

### Contract tests

Verify:

- creative manifests/schema versioning and bounds;
- capability discovery independent from adapter names;
- workflow registry validation;
- job lifecycle/ownership/cancellation;
- asset/provenance/revision lineage;
- effect/approval classification;
- disabled-by-default optional capability behavior.

### Engine adapter tests

Use fake/local mock adapters where possible to verify:

- request parameter validation;
- media-role validation;
- timeout/cancellation;
- output bounds;
- result registration;
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

### Skill routing evals

Cover trigger, route-out, minimal interview, style/identity gate, capability-unavailable response, job chaining, QA behavior, and revision scope without needing a live generation engine for every scenario.

### Production fixture tests

Repository fixtures should test **contracts**, not subjective art quality. Examples:

- Character Pack can express canonical/interpreted views;
- Asset Manifest dependencies resolve;
- Shot Manifest references existing project assets;
- QA finding lineage resolves to one revision;
- Blender character inspection can relate mesh/UV/rig/material/animation state;
- a contained user reference can reach Blender import;
- export/handoff bundle contains required project metadata.

### Manual/operator quality acceptance

Actual anime quality requires operator-enabled engines/Blender and model-side visual judgment. Manual acceptance is allowed to evaluate aesthetics, identity consistency, motion, and final render quality, but it must not substitute for deterministic security/contract tests.

## Failure handling and rollback

- **Bad generation direction:** return to accepted Style/Character revision; do not mutate it in place.
- **Identity/style drift:** reject candidate and retry/revise bounded fields; preserve parent lineage.
- **Bad generated mesh:** discard bootstrap candidate and use another bootstrap/manual blockout; never force a poor mesh through rigging.
- **Topology/rig regression:** restore managed Blender checkpoint.
- **Engine adapter outage:** retain project/job state; fail with capability-specific diagnosis; do not silently switch to an adapter with materially different semantics/cost without policy/user approval.
- **Capability unavailable:** report the missing operator capability/setup; continue independent planning/state work where possible.
- **Unsafe media/path:** reject before engine/Blender sees it.
- **Skill regression:** roll back skill/reference change independently from engine runtime where contracts permit.
- **Model quality regression:** change adapter/model selection policy behind the semantic capability contract; do not rewrite all anime skills.
- **Long-form failure:** fall back to the last passing milestone; never broaden directly from a failing 5-second action to a full episode.

## Alternatives rejected

### Rewrite Blender into a “Higgsfield clone”

Rejected because Blender is a DCC/execution backend, while Higgsfield's valuable architecture lives above execution: skills, state, routing, workflows, QA, and job orchestration.

### Proxy or wrap Higgsfield MCP/CLI

Rejected because the user explicitly requires no Higgsfield account/runtime dependency, and it would keep auth, backend availability, billing, and model behavior outside Masih Awam ownership.

### Copy Higgsfield skill text wholesale

Rejected. The public skills are useful reference material, but Masih Awam needs anime-specific first-party methodology aligned to its own runtime/security contracts. Learn the structure/patterns; author and evaluate the actual product guidance here.

### Put every creative operation behind `terminal_exec`

Rejected because credentials, effects, long-running jobs, media results, capability discovery, and Blender host authority need typed first-class boundaries.

### Expose arbitrary ComfyUI workflow JSON from the model

Rejected for the first release because custom nodes/workflows can become a broad execution/supply-chain boundary. Prefer curated reviewed workflow IDs plus validated parameters.

### Add hundreds of Blender atomic tools

Rejected because a compact inspection/preview/I/O/recovery surface plus privileged `bpy` authoring and strong guidance gives more production flexibility with a smaller contract.

### Train a character model before proving reference-based consistency

Rejected as premature complexity. Character Pack + reference workflow is the baseline; training is added only when measured evals justify the cost/complexity.

### Target a full anime episode first

Rejected because it hides failures across identity, 3D asset readiness, rigging, motion, continuity, rendering, and audio. Milestone M6 is the first integrated quality gate.

## Risks

### RISK-01 — Scope explosion

Creative production spans many domains. Mitigation: milestone ladder and strict M1 -> M6 progression; promo/site/game parity is later and non-blocking.

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

## Final acceptance criteria

Plan 069 implementation is complete only when:

1. Masih Awam has no runtime dependency on Higgsfield services, auth, CLI, MCP, model IDs, or hosted state.
2. Creative skill routing, progressive references, mode-specific interviews, explicit chaining, capability discovery, jobs, visual QA, and surgical revision cover the operating patterns identified in the Higgsfield benchmark.
3. One canonical creative skill ownership path exists in the repository.
4. Creative Project, Character, Style, World, Asset, Shot, Audio, QA, and revision-lineage state are versioned and contained.
5. Capability/workflow discovery is independent from hard-coded engine/model names.
6. Generation jobs are bounded, cancellable/retrievable, owner/project scoped, and return stable output assets.
7. User/generated media can safely materialize into production workspaces with provenance.
8. At least one reference-aware anime image path reaches M1/M2 without Higgsfield.
9. Character identity/style can be reused across multiple stills and revisions.
10. Blender is implemented as the retained 11-tool first-class optional capability with loopback-only bridge and truthful host-authority semantics.
11. One character reaches production-ready-enough mesh/UV/material/hair/clothing state for the benchmark.
12. The character has a reusable body rig plus initial facial controls and passes representative deformation QA.
13. A 3–5 second animation passes structural and temporal review.
14. A 10–30 second anime scene passes shot-manifest, continuity, visual, temporal, render, and export review.
15. Final output and reusable project/character assets remain in contained workspace paths and can be resumed by a fresh session.
16. A second-project falsification shows the pipeline is not hard-coded to the first demo.
17. No arbitrary model-supplied engine endpoint/workflow graph, unrestricted Blender host/path, or secret-bearing generic terminal fallback bypasses the reviewed capability boundaries.
18. Generate/render/export does not silently publish/deploy content.
19. Skill evals and repository-native contract/security tests cover routing and failure behavior.
20. Relevant focused tests and `pnpm guardrail:fast` / affected stack full gates / `pnpm guardrail:full` pass before closure.
21. Operator-only actions such as GPU model installation, Blender/add-on setup, relay restart, deployment, or external publishing are reported explicitly and are not performed implicitly by the implementation agent.

## Current execution state

As of 2026-09-12:

- `origin/main` is `e13bd38666ec4cb100ae713bd8272426db209d0e`;
- Plan 069 did not yet exist on `origin/main`; 069 is the next unused numeric plan there;
- the earlier Blender-only Plan 069 lived on the local `release/ai-tools-v0.0.15` branch and had no Blender source implementation started;
- the earlier Blender design was reviewed and its core bridge, security, 11-tool surface, anime/3D workflow guidance, attachment-ingress requirement, QA, asset I/O, and checkpoint decisions are retained inside this broader plan;
- the user's product direction now explicitly changes the top-level target from “first-class Blender MCP” to an **anime-first Creative Production Platform**, with Blender as one production engine;
- Higgsfield is explicitly a public behavioral benchmark only, not a runtime dependency;
- the public Higgsfield skills/CLI behavior was re-audited on 2026-09-12 for the 1:1 comparison above;
- no implementation files were changed as part of this planning rewrite.
