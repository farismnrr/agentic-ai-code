# Plan 011: Fix chat prompt not sticking to the bottom

## 1. Problem
Feedback on `/chat/[id]` (Flash PreviewTools): the `UChatPrompt` form
(`UDashboardPanel`'s `#footer`) isn't sticky at the bottom — it gets cut off.

## 2. Root cause
`UDashboardGroup` is `fixed inset-0 flex overflow-hidden` (viewport-pinned,
clips overflow). `UDashboardPanel`'s root carries `min-h-svh` unconditionally
(`.nuxt/ui/dashboard-panel.ts`), on top of the `flex-1` sizing variant.

`min-h-svh` overrides the flex item's default `min-height: auto`, forcing the
panel to be *at least* one small-viewport-height tall even when the actual
flex space available inside the fixed group is smaller (mobile browser
chrome, URL bar, dynamic toolbars). When that happens the panel grows past
its container, `UDashboardGroup`'s `overflow-hidden` clips the excess, and
since the footer sits after the scrollable `body` in flow, it's the part
that gets clipped off-screen — not "unstuck", just pushed past the visible
edge.

## 3. Fix
Override the `dashboardPanel` theme slot in `app/app.config.ts` so `root`
uses `h-full` instead of `min-h-svh`, matching the `flex-1` variant's intent
(fill the fixed-height parent exactly, never exceed it). This is the
supported Nuxt UI mechanism for adjusting default theme classes — no
component patching.

## 4. Files
- `app/app.config.ts` — add `ui.dashboardPanel.slots.root` override.

## 5. Verify
- [ ] `npm run dev`, open `/chat/[id]` with a long message thread.
- [ ] Chrome DevTools mobile emulation (iPhone SE / 320px), confirm the
      prompt form stays pinned to the bottom, not cut off, with the on-screen
      keyboard toggled.
- [ ] Confirm desktop layout (sidebar + panel) unaffected.
