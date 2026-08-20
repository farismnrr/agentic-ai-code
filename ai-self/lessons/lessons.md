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

- Date: 2026-08-16
- Context: Making the Bubblewrap-backed Masih Awam MCP relay usable as a full local coding environment through an external MCP bridge.
- Lesson: Separate HTTP identity, host-daemon access, and developer toolchains into explicit trust surfaces. Non-browser MCP clients may legitimately omit Origin, but any present Origin must match the configured browser Origin and Host must remain an exact local or operator-allowlisted authority. Expose Docker/Tailscale only through opt-in socket binds. Do not inherit the login-shell PATH; add reviewed runtime directories with `--toolchain-path` and verify capabilities from inside MCP after restart.
- Applies to: Local coding relays, MCP bridges, sandboxed coding agents, and developer environments using version-managed Node/Rust/Bun toolchains.
- Action taken: Documented the relay coding profile, explicit Host/Origin behavior, Docker/Tailscale boundaries, safe toolchain PATH configuration, direct-argv shell semantics, and an MCP-side smoke-test checklist.

- Date: 2026-08-19
- Context: Reconciling explicit workspace authorization with a broader single-owner execution ceiling.
- Lesson: Keep the filesystem ceiling and active workspace authority separate. `--execution-root` is the hard maximum boundary, `--dir` is the primary authorized workspace, and sibling projects require explicit bounded `workspace_add` authorization. Setting both to `$HOME` intentionally authorizes the whole home tree and should not be the default when an explicit allowlist is desired. Toolchain mounts may live elsewhere beneath the execution ceiling without becoming workspaces.
- Applies to: Masih Awam MCP local coding relay and similar sandboxed multi-project coding environments.
- Action taken: Split execution-boundary vs workspace-allowlist semantics in core containment, updated Bubblewrap/Git/file-tool integration, and corrected operator/agent documentation.

- Date: 2026-08-16
- Context: Adding native read/search/edit/write tools to a filesystem-contained MCP coding relay.
- Lesson: Treat canonical path containment as validation-time evidence, not mutation safety. For security-sensitive traversal and writes, keep one shared path contract, then use stable directory descriptors with no-follow opens, revalidate final entry identity at operation time, and commit writes through same-directory temporary files with explicit atomic/no-clobber semantics. When a broad regression check fails outside the changed surface, compare the same behavior against the baseline branch before expanding scope; unchanged baseline failures should be reported as unproven/pre-existing rather than silently “fixed” or falsely marked green.
- Applies to: Sandboxed coding relays, filesystem MCP tools, local agents, and any service performing contained native filesystem mutation.
- Action taken: Workspace v1 now shares one execution-root containment foundation, uses descriptor-based traversal and atomic mutation guards, includes adversarial MCP verification, and records the pre-existing outbound `http_fetch` limitation separately from Plan 038 regression results.

- Date: 2026-08-18
- Context: Adding action-level agent/tool observability on top of the existing Plan-035 OpenTelemetry and structured-logging pipeline.
- Lesson: A logger sanitizer is not automatically an OpenTelemetry span sanitizer. Any code that calls `startActiveSpan(..., { attributes })` directly can bypass the logger chokepoint unless it explicitly passes the same allowlist sanitizer first. Keep semantic telemetry low-cardinality and content-free, and reuse one sanitizer contract across logs and direct span attributes instead of creating a second observability policy.
- Applies to: Masih Awam server observability, MCP/tool instrumentation, and future direct OpenTelemetry spans.
- Action taken: Routed MCP span attributes through `sanitizeAttributes`, extended the existing allowlist only with reviewed semantic agent/tool fields, and added deterministic confidentiality acceptance for the new vocabulary.

- Date: 2026-08-18
- Context: Hardening first-party agent approval/subagent/background presentation during Plan 039J independent closure review.
- Lesson: Presentation confidentiality must fail closed at the final presentation boundary. Prompt instructions and broad key-name allowlists are insufficient for model-controlled or user-controlled free-form text; hide arbitrary free-form approval values by default, re-sanitize child/background results after later enrichment such as Git evidence, never surface raw dynamic errors, and treat POSIX, Windows-drive, UNC, and embedded absolute paths as sensitive presentation data.
- Applies to: Agent/tool approval cards, subagent/background result cards, MCP-derived UI summaries, and any future model-controlled presentation surface.
- Action taken: Narrowed approval scalar rendering, added final-boundary structured child/background sanitization with size limits and secret/path redaction, removed raw delegated-task/error rendering, and added cross-platform confidentiality canaries to the deterministic 039J verifier.

- Date: 2026-08-20
- Context: Reviewing and remediating delegated coding-CLI execution through a Bubblewrap-backed MCP relay.
- Lesson: Do not treat adapter strings or a shared sandbox helper as proof that delegated agents are safe and usable. Validate provider argv against the actually installed CLI interface, translate each authority class at the invocation owner instead of OR-ing unrelated global flags, scope delegated mounts to the selected writable workspace, and make automatic fallback observe all writable state or fail closed when that cannot be proven. Acceptance should execute fake provider CLIs through the real sandbox/job path so timeout state, retained output, ignored-file mutation, network isolation, sibling-workspace visibility, and fallback ordering are tested together.
- Applies to: MCP agent delegation, sandboxed coding CLI adapters, quota/auth fallback chains, and other subprocess orchestration that combines mutable workspaces with provider failover.
- Action taken: Promoted capability-filtered delegation into the Primary fast path without widening the public catalog, updated the Codex adapter to the installed noninteractive interface, separated terminal/agent network authority and sibling mounts, replaced Git-status fallback fingerprints with bounded metadata snapshots, preserved timeout/cancel output semantics, and added real Bubblewrap provider-fixture acceptance.
- Lesson: When a single-owner coding relay needs host GitHub authentication, do not unmask the whole home directory or copy tokens. Use an explicit opt-in that mounts only GitHub CLI config and Git user config read-only into ordinary terminal sandboxes, keep generic credential stores protected, and keep network authority as a separate capability. Apply the narrow exception after generic credential masks so the opt-in actually takes effect.
- Applies to: Sandboxed MCP coding relays that need to reuse an operator's existing `gh`/HTTPS Git login.
