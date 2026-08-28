# Plan 055 — Human-readable Workspace Activity Actions

**Status:** CLOSED / VERIFIED
**Created:** 2026-08-28

## Problem

Workspace Logs identified actor/tool/status but often could not answer the operator's primary question: what concrete action was performed. Relay activity intentionally excluded raw arguments, leaving terminal calls such as `terminal_exec` with only a generic target/result.

## Goal

Record and present a bounded, credential-redacted **specific action + specific result** for every tool. List cards show the concrete input/action and a concise result; opening an activity shows the bounded full result detail. Raw request objects, prompts, auth, environment, and unbounded result payloads remain forbidden.

## Implementation

- Extend `activity.event.v1` presentation metadata with optional `action` (max 256 chars) and bounded multiline `result_detail` (max 8192 chars).
- Derive concrete tool-specific actions at the relay execution boundary: terminal command/args, file path/range/edit count, actual text/file-search query, patch targets, Git operation/ref, HTTP method + host/path, task ID, and bounded non-sensitive scalar arguments for the remaining tools.
- Treat `terminal_job_start` as the command it launches (for example `cat file.txt`), not as a generic job operation.
- Derive a concise result from the actual tool envelope while retaining a bounded redacted full result detail for the click-through view. Terminal summaries prefer `Exit: N · <first useful stdout/stderr line>`.
- Normalize workspace absolute paths and deterministically redact credential-shaped values from both action and result detail before journaling/export.
- Persist/map action, concise result, and result detail through Nuxt ingest/read model and expose them in shared API types.
- Make Workspace Logs action/result-first: the list card is concrete action + concise result; the drawer is concrete action + bounded full result. Actor/tool/evidence remain technical details. Historical rows without action explicitly say input was not recorded rather than inventing a generic action.

## Verification

- Focused Rust activity tests prove terminal commands, terminal-job commands, non-terminal concrete arguments, and credential redaction.
- Web activity contract test covers `action` + `result_detail` ingestion/mapping/UI.
- `pnpm guardrail` must pass across web + Rust after this refinement.
- Production Nuxt build passed with a build-only dummy session password, the Nuxt container was deployed and returned HTTP 200, and the release `ai-tools` artifact built successfully. The artifact was atomically installed at the operator binary path and its SHA-256 matched the built artifact. The running relay process was not restarted, signaled, or modified through systemd; refined relay-side fields activate on the next operator-controlled process start.
