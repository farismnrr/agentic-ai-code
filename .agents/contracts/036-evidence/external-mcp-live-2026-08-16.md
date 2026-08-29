# Plan 036 — Live external MCP evidence (2026-08-16)

This file records only evidence observed in the live external MCP client session. It intentionally excludes tokens, OAuth codes, subjects, credentials, private configuration values, and other secrets.

## Proven in this session

- external MCP client reached the configured `Masih_Awam_MCP` server.
- The connected server exposed `terminal_exec`, `http_fetch`, and `web_search` to the session.
- A non-destructive `terminal_exec` call succeeded against the configured repository workspace.
- The call returned `/home/farismnrr/Projects/MasihAwam/ai-code` as the working directory and reported branch `feat/plan-036-remote-mcp` with a clean working tree at the start of the release task.
- Subsequent read-only repository inspection through the same tool also succeeded.

## What this does not prove

This evidence does **not** independently capture or validate:

- the browser-visible OAuth authorization/callback exchange;
- decoded access-token audience/resource/subject/scope claims;
- hosted Nuxt -> public MCP -> laptop execution;
- cross-user owner-binding negative cases;
- destructive-action approval behavior;
- the full Phase 7/11 negative-case matrix.

Those items remain open in Plan 036 unless separate evidence proves them.
