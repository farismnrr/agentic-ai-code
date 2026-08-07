# 003 — "Instrument": give the product a visual identity

## Context

The UI reads flat because **no design decision has been made yet**. The palette is the Nuxt starter's default green on slate, the typeface is the template's Public Sans, and every radius, shadow and transition is a Nuxt UI default. It isn't under-designed — it's un-designed. Plans 001 and 002 were about behaviour, and behaviour is now there.

This plan gives it one deliberate direction, applied through tokens so the whole app moves at once, then hand-finishes the surfaces people actually look at.

## The direction: Instrument

A precise technical instrument. Cool graphite ground, and **one signal colour that means "live"**.

### Tokens

| Role | Value | Notes |
| --- | --- | --- |
| Ground | `#0B0E14` | graphite, dark mode base |
| Surface | `#151A23` | raised panels |
| Signal | `#4CC2FF` | cyan-blue — the only saturated colour |
| Text | `#E6EAF2` | |
| Muted | `#8A93A5` | |

Type: **Geist** for display and body, **Geist Mono** for labels and data. Both confirmed on Google Fonts, and `@nuxt/fonts` already ships with Nuxt UI — it self-hosts them automatically, no new dependency.

Radius `6px`, small and deliberate. Borders do the work that shadows usually do.

### The signature: colour means liveness

**The cyan is reserved for things that are actually happening.** Streaming text, a running tool call, a connected MCP server, a focused input. Nothing decorative ever uses it. Everything else is graphite and text.

That makes the palette carry information rather than mood — appropriate for a tool whose whole subject is *watching a model work*. It also constrains me: if the cyan starts appearing on a marketing button, the idea is broken.

### Light mode is not an afterthought

The direction is dark-first, and the app has a working light mode. Light is derived, not inverted: the ground becomes near-white with a cool cast, the signal darkens to hold contrast against it (a `#4CC2FF` button on white fails contrast), and borders carry more weight since there's no glow to rely on. **Every phase gets checked in both.**

## Motion

Per the `ui-animation` skill, now installed at `.agents/skills/ui-animation/`:

- **CSS transitions first.** They retarget on interruption; keyframes restart from zero. JS only where CSS genuinely can't.
- **One orchestrated moment**, not scattered effects — scattered motion is what makes a design read as AI-generated.
- 120–160ms for hover and focus; overlays emerge from the control that opened them, not from the centre of the screen.
- Confirmation lands on the control that caused it — the copy button becomes "Copied" rather than firing a toast in the corner.
- `prefers-reduced-motion` respected everywhere.

## Build order

Each phase ends green (`pnpm lint && pnpm typecheck && pnpm audit`) with its own PR into `dev`.

### 1. The token system

- `app/assets/css/main.css` — replace the green scale with a `signal` scale, set `--font-sans: 'Geist'` and `--font-mono: 'Geist Mono'`, define radius, elevation and easing tokens under `@theme`.
- `app/app.config.ts` — point `ui.colors.primary` at the new scale, `neutral` at a cool grey.
- Nothing else changes in this phase. The whole app should visibly shift on token change alone; if a surface doesn't move, it's hardcoding something it shouldn't, and that's worth knowing before hand-finishing anything.

### 2. Landing — the hero is a live demo

The most characteristic thing about this product is watching a reply stream in. So the hero **is** that, driven by the `MockChatTransport` already in `app/utils/mock-transport.ts` — not a screenshot, not a video.

- A self-running conversation that types a prompt, streams a reply, shows a tool call, and loops. Paused under `prefers-reduced-motion`, and paused off-screen so it isn't burning frames behind the fold.
- The rest of the page gets scroll-reveal on section entry, and that's the whole motion budget for the page.
- Rework the feature cards, pricing and FAQ against the new tokens.

### 3. Auth surfaces

Login and register are the second thing anyone sees and are currently a plain card. Give them the ground treatment, mono labels, and a considered focus state — focus is where the signal colour earns its keep.

### 4. Chat surface

The screen people live in.

- Message treatment: user and assistant should be distinguishable by structure, not two coloured bubbles.
- Streaming caret in signal cyan — the one place motion and colour agree.
- `ChatToolCall` and `ChatToolApproval` restyled as instrument readouts, with mono for tool names and arguments.
- Sidebar: denser, quieter, with the active row carrying the only accent.

### 5. Settings and empty states

Bring the four settings tabs and the empty states onto the system. Empty states are an invitation to act, not an apology.

### 6. Audit

- Both themes, every route.
- `prefers-reduced-motion` honoured.
- Visible keyboard focus everywhere — the signal colour makes this easy to get right and embarrassing to get wrong.
- Mobile down to 375px.
- Raw-palette grep stays clean; semantic tokens only.

## Conventions to hold

- **Semantic tokens only.** The point of phase 1 is that phases 2–6 barely touch colour.
- Read `.nuxt/ui/<component>.ts` or the `nuxt-ui` MCP for real slot names before overriding a component's `ui` prop — guessing produces classes that silently do nothing.
- `pnpm audit` at zero before each merge.

## Verification

Per phase: `pnpm lint && pnpm typecheck && pnpm audit`, and `pnpm build` before the last PR.

**Including the check that plan 001 skipped:** after loading each route in a real browser, grep the dev-server log for `Failed to resolve component`, `Hydration`, and `[console.error]`. See `.agents/memories/verify-in-a-browser.md` — a page that renders is not a page that works.

By eye, at http://100.99.88.53:3333, in **both themes**:

1. Landing hero streams a reply on load, and stops when the tab is backgrounded.
2. Set `prefers-reduced-motion` — the hero holds a static final frame; nothing else animates.
3. Tab through login: focus is obvious on every control.
4. Send a message: the caret pulses in signal cyan while streaming, and stops when it's done.
5. Trigger a tool: the approval dialog emerges from the message, not the screen centre.
6. Copy a message: the button confirms in place.
7. Light mode on every route — no washed-out cyan, no invisible borders.
8. 375px wide: nothing overflows, sidebar collapses.

## Out of scope

No backend. No custom components replacing Nuxt UI ones — tokens and the `ui` prop only. No new animation dependency unless phase 2 proves CSS can't drive the hero, and if it comes to that I'll say so rather than adding it quietly.

## On completion

Copy to `.agents/plans/003-instrument-design.md` and tick phases there. Record the signal-colour rule in `.agents/knowledge/conventions.md` — it's a constraint future work has to hold, not a one-off styling choice.
