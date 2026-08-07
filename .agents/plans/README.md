# Plans

Implementation plans for multi-step work, one Markdown file per effort.

**Naming: `NNN-kebab-case.md`** — a zero-padded 3-digit sequence number, then a short descriptive name (`001-chat-ui.md`, `002-auth-flow.md`). Take the next unused number; never reuse one, even after a plan is deleted. The number is the stable handle — it makes plans easy to reference in conversation ("lanjut 002") and keeps them in creation order in the listing.

Write a plan here when a task spans several sessions or several files, so the next agent can pick it up without re-deriving the approach. A plan should state the goal, the steps in order, the files each step touches, and how to verify it worked. Mark steps done as you go.

**Plan mode output lands here too.** When a plan is produced in plan mode, write it to this folder under the next number — don't leave it in the harness's scratch location where it's invisible to the repo.

Keep shipped plans; move them to the Done list below rather than deleting, so the numbering stays meaningful and past decisions stay readable.

## In Flight
*None active.*

## Completed
- [006-error-handling.md](006-error-handling.md) — centralize server error handling (RFC 9457 Problem Details) and audit every 4xx/5xx to match its real failure scenario.
- [005-backend-auth.md](005-backend-auth.md) — real backend auth (cookie session, Postgres, OAuth, email verification) then chat/settings/MCP persistence.
- [004-ui-responsiveness.md](004-ui-responsiveness.md): Audit and resolve responsiveness on Mobile S through Desktop 2K across all UI pages.

- [001-chat-ui.md](001-chat-ui.md) — external MCP client-like AI chat UI, frontend only
- [002-landing-auth-interactive.md](002-landing-auth-interactive.md) — landing → login → app, and closing the interaction gaps
- [003-instrument-design.md](003-instrument-design.md) — "Instrument": give the product a visual identity
