---
name: premium-landing-page
description: Use when designing, specifying, reviewing, or implementing a premium landing page that must feel distinctive rather than generic AI/SaaS, especially when cinematic scroll storytelling or product-specific motion is required.
license: MIT
---

# Premium Landing Page

Create landing pages with a deliberate visual thesis, product-specific storytelling, and motion that supports narrative rather than decorating every section.

## Scope

Use this skill for:

- premium marketing or product landing pages;
- landing-page sections inside a broader application prototype;
- non-generic visual direction and composition;
- cinematic scroll storytelling;
- signature product-specific interactions;
- translating an existing product design language into a more expressive public-facing brand layer.

Do not use this skill as the primary authority for authenticated application UX, long-form learning screens, dashboards, or routine component interaction. Those surfaces should remain governed by their product/design-system rules.

## Required companion skills

Load only what the task needs:

- `frontend-design` for visual thesis, typography, layout, composition, and anti-template critique;
- `ui-animation` for motion purpose, easing, interruption, accessibility, and interaction motion;
- `gsap-core` for tween fundamentals and reduced-motion handling;
- `gsap-timeline` for coordinated sequences;
- `gsap-scrolltrigger` for pinned/scrubbed/scroll-driven storytelling;
- `gsap-frameworks` for Vue/Nuxt lifecycle and cleanup when applicable;
- `gsap-performance` for jank/performance review;
- `gsap-plugins` only when a specific effect truly needs SplitText, Flip, MorphSVG, ScrollSmoother, etc.

Do not load every companion skill by default. Keep the active set minimal.

## Workflow

### 1. Ground in the actual product

Before designing, inspect the target project's PRD, design spec, current UI, copy, assets, and brand/design-system evidence. Identify:

- the product's actual audience;
- the single job of the landing page;
- the product's visual language that must remain recognizable;
- the public-facing expression layer that may become more theatrical;
- product-specific concepts that can become the visual/motion motif.

Never invent a disconnected visual theme just because it looks premium.

### 2. Define one visual thesis

Write one concise thesis covering:

- typography character;
- composition/layout character;
- color discipline;
- use of depth or layering;
- one memorable signature motif.

The signature motif must come from the product itself. Examples: learning progression, course-path assembly, knowledge layers, transformation from overview to focused lesson, or another domain-specific metaphor.

Reject visual directions that rely primarily on generic AI/SaaS defaults such as:

- gradient blobs or mesh gradients as the main identity;
- floating glass cards;
- arbitrary 3D chrome objects;
- giant centered headline + two CTA + fake metrics formula;
- meaningless orbiting shapes;
- excessive pills;
- identical fade-up sections;
- random stagger applied everywhere.

### 3. Separate brand theatre from product clarity

A premium public landing page may be editorial, cinematic, and compositionally bold. Authenticated/product surfaces should usually remain calmer and task-oriented.

When showing product UI inside the landing page:

- preserve its real design language;
- let cinematic composition frame or transition it;
- do not redesign the app into a different brand merely for the hero.

### 4. Build a scroll narrative before implementation

Define the landing page as a sequence of narrative beats, not a stack of sections.

For each beat specify:

- user question being answered;
- visual focus;
- copy role;
- motion purpose;
- scroll behavior;
- transition into the next beat;
- reduced-motion fallback.

A typical structure may include:

1. thesis/hero;
2. problem or learner context;
3. signature product transformation;
4. product proof / interface reveal;
5. learning journey or feature story;
6. final conversion moment.

This is a template for reasoning, not a mandatory section count.

### 5. Design one signature motion moment

At least one moment should be memorable and product-specific.

It must:

- communicate something real about the product;
- have a clear start and end state;
- preserve readable content while active;
- degrade gracefully on small screens;
- have a reduced-motion equivalent;
- avoid scroll-jacking;
- avoid requiring continuous high-cost animation when off-screen.

Prefer one strong orchestrated moment over many unrelated effects.

### 6. Choose implementation by motion need

Use the lowest-complexity mechanism that produces the intended result:

- CSS transitions for local hover/focus/state changes;
- GSAP core for coordinated DOM animation;
- GSAP timeline for multi-phase choreography;
- ScrollTrigger for scroll-linked progress, pinning, scrub, or section orchestration;
- plugins only for effects that cannot be expressed cleanly otherwise.

For Nuxt/Vue, create animations only after mount, scope selectors with `gsap.context`, and revert on unmount.

### 7. Performance and accessibility gate

Before approval:

- favor transform/opacity over layout properties;
- do not leave `will-change` everywhere;
- ensure off-screen loops stop;
- keep ScrollTriggers bounded and intentional;
- test narrow/mobile layouts separately rather than merely shrinking desktop choreography;
- support `prefers-reduced-motion` with instant or low-motion state equivalents;
- preserve semantic reading order and keyboard navigation regardless of visual layering;
- ensure pinned content does not trap or obscure focus.

### 8. Anti-generic critique pass

Before implementation and again before completion, ask:

- Could this exact page be re-labeled for an unrelated SaaS product without changing the visuals?
- Is the signature idea grounded in the actual product?
- Are multiple sections using the same entrance treatment?
- Is motion carrying narrative information or merely creating activity?
- Is typography doing meaningful brand work?
- Is there one clear memorable idea, or a pile of trends?

If the page can be trivially re-skinned for another product, revise the visual thesis before polishing details.

## Asset-backed media workflow

When a premium landing page will receive generated image/video assets:

1. Research motion/3D references broadly before choosing media direction; references may come from unrelated industries when their design language fits the product.
2. Lock one primary direction and document explicit subject substitution: preserve useful composition/motion grammar while replacing reference subject matter with project-specific content.
3. Inspect the actual target HTML geometry before prompting. Record source-generation ratio separately from HTML display ratio, plus background, radius, crop, `object-fit`, `object-position`, safe region, and surrounding UI.
4. Choose the smallest useful asset set. Put all final production media under `assets/` and reference final filenames from HTML before final media exists.
5. For composition-sensitive video, default to image-first: still prompt → approved still → video-from-image prompt. The video prompt animates the still; it does not redesign it.
6. Keep authored media motion separate from browser interaction. Media supplies visual storytelling; HTML/CSS/JS owns scroll orchestration, masks, layout, state, interaction, responsive crop/scale, and reduced-motion behavior.
7. Create valid non-zero placeholders at the final paths with production geometry **and production ownership**. If generated media replaces an older static illustration/mockup, the placeholder must immediately become the primary visual in that exact final slot; remove/refactor the legacy visual instead of hiding media behind it. Do not preserve old DOM artwork above/below the placeholder unless that layered composition is explicitly part of the approved final design.
8. Treat replacement as a file-content-only operation: final media must not require changing DOM elements, z-index, visibility rules, crop, component geometry, or switching away from a legacy/static layer.
9. Verify the live placeholder template in a real browser before handoff, including media ownership/layering, playback/loop, crop, responsive layouts, reduced-motion fallback, layout shift, controls, and runtime errors. Reuse existing Playwright/browser tooling before installing anything.

## PRD / design-spec output requirements

When the task is specification-first, ensure the PRD or design document explicitly covers:

- premium landing-page goal and non-goals;
- relationship between public brand expression and authenticated product UI;
- anti-generic visual constraints;
- signature motif;
- scroll narrative/storyboard;
- signature motion interaction;
- allowed/prohibited motion;
- responsive simplification;
- reduced-motion behavior;
- performance requirements;
- resource-gathering expectations;
- acceptance criteria that distinguish authored design from generic AI output.

## Validation

A successful result should satisfy all of these:

- visual direction is specific to the product;
- there is one coherent signature motif;
- scroll motion has narrative purpose;
- application UI remains recognizable and usable;
- motion is accessible and responsive;
- implementation can explain why GSAP/ScrollTrigger is used where it is used;
- no section exists solely to showcase an effect;
- the page still communicates correctly with reduced motion enabled.

For deliberately dependency-free, single-file prototypes, do not install a browser-testing framework solely for validation when a headless Chromium/Chrome binary is already available. A small temporary Chrome DevTools Protocol harness is sufficient when no stronger project requirement exists. **If the user/project explicitly requires Playwright, that requirement wins:** search for and reuse an existing project/workspace/global Playwright installation first, and only install when genuinely unavailable. Keep validation harnesses ephemeral; they are not part of the prototype deliverable.
