---
name: motion-explainer-video
description: Use when creating, extending, reviewing, or debugging browser-rendered animated explainer videos, especially reusable dual-orientation YouTube/vertical explainers built from HTML/SCSS/vanilla JS segments.
license: MIT
---

# Motion Explainer Video

Use this skill for programmatic explainer videos where animation is rendered in the browser and each visual beat must explain the narration. For low-level easing/transition craft, also use the installed `ui-animation` skill as a supporting reference.

## Core principle

Do not build an animated slide deck. Build a visual explanation.

Every narration beat must produce a meaningful visual action or state change that represents the sentence: search rays scan sources, evidence is selected, chunks move into prompts, embeddings become vectors, and pipeline state travels through connectors. Fades alone are not an explanation.

## Story workflow

1. Establish one concrete audience promise.
2. Write the story map before polishing visuals.
3. Start general and concrete; introduce jargon only after the mental model exists.
4. Let runtime follow the story. A rough 1–3 minute range is fine when requested; never force an exact duration unless the user explicitly requires one.
5. For YouTube-style explainers:
   - open with a 5–8 second cold hook based on tension, surprise, or a useful question;
   - avoid logo-only intros;
   - end with a short recap plus a natural next-topic hook;
   - a subscribe CTA may support the outro but must not replace the next-topic value.

Recommended curve:

`hook → concrete failure/example → plain-English model → conceptual mechanics → technical system → recap → next-topic hook`

## Artboard and responsiveness

Treat the video as a deterministic fixed artboard and scale it as one unit.

For the current explainer format:

- landscape artboard: **1280×720 (16:9)**;
- portrait artboard: **720×1280 (9:16)**;
- orientation controls switch the actual video composition, not only the outer viewer surface;
- both orientations share one story, timeline, captions, template, and semantic motion;
- base/landscape layout stays deterministic, while portrait gets explicit composition overrides;
- test both compositions across portrait and landscape browser viewports;
- keep composition-safe zones explicit per orientation;
- use a dedicated caption shelf instead of placing captions opportunistically inside scene layouts.

If a project provides a dual-orientation format preset, reuse it rather than repeating dimensions.

## Architecture

Prefer this dependency direction:

```text
main
 ├─ player/core
 └─ videos registry
     └─ video
         └─ segments
```

Rules:

- shared player code owns playback, artboard switching/scaling, seeking, captions, orientation state, and timeline compilation;
- each video owns metadata, story order, and video-level SCSS;
- each segment owns one narrative responsibility, one shared template/motion model, base/landscape SCSS, and portrait composition overrides;
- segments may use generic core helpers but must not depend on player internals or sibling segments;
- split a segment into `template.js` / `motion.js` only when the entry file becomes hard to scan;
- prefer SCSS tokens/mixins for genuinely repeated primitives; do not create abstractions for accidental visual similarity;
- keep files small enough to scan, but do not split mechanically just to satisfy a line-count target;
- do not duplicate a segment just to support portrait; use the same segment with `portrait.scss` and `meta.orientation` only where motion geometry genuinely differs.

## Segment contract

A typical segment:

```js
export default defineSegment({
  id: "retrieve",
  duration: 12,
  captions: [{ start: 0, end: 4, text: "..." }],
  template: `...`,
  render(root, progress, meta) {
    const portrait = meta.orientation === "portrait";
  },
});
```

Keep base/landscape layout in `styles.scss` and true portrait recomposition in `portrait.scss`. Prefer geometry-derived motion over duplicated orientation-specific constants when practical.

Total video duration must be derived from segment durations, not duplicated in a global constant.

## Motion rules

- Use spatial continuity: objects should move from where the viewer last saw them.
- Prefer transforms and opacity for frame-by-frame motion.
- Keep text readable while progress indicators, beams, or particles move nearby.
- Fix geometry before using z-index as a collision workaround.
- Mark intentional overlaps explicitly (for example `data-overlap-ok`) so validators can distinguish designed motion from bugs.
- When text must never be obscured, mark readable/occluder roles when the project validator supports them.
- Use progressive disclosure: reveal only the part of a technical system currently being explained.
- Search/retrieval visuals must show the act of searching before selection. A scanner should visibly inspect candidate sources, then settle on and highlight the chosen source; do not draw a static line directly to the answer and call it retrieval.
- Avoid simultaneous competing motion in unrelated regions.

## Captions

- Captions are supporting narration, not the primary graphic.
- Keep them centered, restrained, and normally one or two lines.
- Reserve a fixed caption safe zone for the entire video format.
- Critical visual elements must not enter the caption shelf.

## Reuse for a second video

When the repository contains an authoring guide or scaffold command, use it first. In the current motion-explainer project:

```bash
npm run create:video -- <kebab-id> "Video Title"
```

Then:

1. write/update the new video `STORY.md`;
2. replace scaffold beats with the actual story segments;
3. register the video if the repository uses a static registry;
4. preserve the same format preset, caption shelf, SCSS boundaries, and validation contract unless the user explicitly requests a different format.

## Validation

Do not finish based on one screenshot.

Minimum checks:

1. syntax/format/build checks;
2. deterministic seek to multiple timestamps, including every segment boundary;
3. browser runtime errors;
4. critical elements escaping the artboard;
5. caption collisions;
6. unexpected critical-element overlaps;
7. the entire timeline in both portrait and landscape artboards, plus shared orientation controls when present;
8. targeted geometry checks for any specific collision the user reported.

For the current project, use:

```bash
npm run check
npm run validate:layout
```

A targeted user-reported collision is not fixed until a check proves that exact geometry no longer intersects throughout the affected time range.

## Review checklist

- [ ] Hook earns attention quickly without a logo-only intro.
- [ ] Story goes general → conceptual → technical.
- [ ] Every caption is visualized by motion or a meaningful state change.
- [ ] Landscape and portrait artboards are both deterministic, with real composition changes rather than surface-only preview changes.
- [ ] Captions remain readable and unobstructed.
- [ ] No unintentional critical-element overlaps.
- [ ] Total runtime is story-driven.
- [ ] Segment and SCSS boundaries remain maintainable.
- [ ] Validation passes before handoff.
- [ ] Outro reinforces the mental model and points to a useful next topic.
