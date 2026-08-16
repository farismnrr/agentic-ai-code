# Reusable Lessons

Record only durable, reusable lessons discovered from completed work or user corrections.

## Entry format
- Date:
- Context:
- Lesson:
- Applies to:
- Action taken:

- Date: 2026-08-16
- Context: Browser-rendered motion explainers built with vanilla HTML/SCSS/JS.
- Lesson: Treat the video as a fixed artboard that scales uniformly; do not mix responsive canvas sizing with fixed-position child UI. Keep player/caption/timeline logic in a reusable core, keep each narrative segment in its own module and stylesheet, derive total runtime from segment durations, reserve explicit caption-safe zones, and validate every timestamp for out-of-frame and unexpected element overlap. Mark only intentional motion overlaps explicitly.
- Applies to: Short-form explainer videos, animated diagrams, and browser-rendered video compositions.
- Action taken: Refactored the explainer system into fixed-format artboards that scale uniformly, modular segment architecture, story-driven duration, and automated Playwright overlap validation; the current YouTube preset is 1280×720 (16:9).

- Date: 2026-08-16
- Context: Dual-orientation browser-rendered motion explainers.
- Lesson: Viewer orientation and video composition are different concerns. If the user asks for portrait and landscape output, changing only the outer preview surface is insufficient; switch the actual artboard (for example 1280×720 ↔ 720×1280), provide a real per-orientation composition, keep one shared story/timeline/template, and expose orientation to motion code only where geometry changes. Validate the entire timeline in both artboards, not just the orientation control state.
- Applies to: YouTube/short-form explainers, responsive motion graphics, and browser-rendered video tooling.
- Action taken: Reworked the motion-explainer system from letterboxed portrait preview to true dual-orientation artboards with per-segment portrait SCSS and dual-orientation full-timeline validation.

- Date: 2026-08-16
- Context: Portrait recomposition of dual-orientation browser-rendered motion explainers.
- Lesson: A true portrait artboard can still feel wrong when it only repositions landscape-sized objects. After switching to 720×1280, explicitly retune visual density: illustration scale, card/diagram width, typography, padding, gaps, and negative space. Collision-free is necessary but not sufficient; compare representative element proportions in portrait before declaring the composition done.
- Applies to: Dual-orientation motion explainers and responsive browser-rendered video compositions.
- Action taken: Added a portrait density pass across all RAG segments, shared caption/background/play-overlay styling, and updated the reusable motion-explainer workflow.

- Date: 2026-08-16
- Context: Extending the Bubblewrap-backed MCP coding relay with host Docker debugging.
- Lesson: Treat Docker daemon access as an explicit operator trust expansion, not as ordinary command allowlisting. Keep Docker denied by default, require an opt-in flag/environment setting, bind only the selected Unix socket, support configurable/rootless socket paths, and document that daemon access can escape the filesystem sandbox. Preserve direct-argv terminal semantics unless a shell-aware policy parser exists; silently wrapping commands in `sh -lc` would bypass executable-level deny rules.
- Applies to: Sandboxed local-development relays, coding agents, and terminal execution services that expose privileged host daemons.
- Action taken: Added opt-in Docker configuration and socket plumbing with default-deny policy/tests, clarified terminal shell semantics, and kept the relay self-update boundary intact.
