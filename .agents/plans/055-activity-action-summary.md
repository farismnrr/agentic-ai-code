# Plan 055 — Human-readable Workspace Activity Actions

**Status:** CLOSED / VERIFIED
**Created:** 2026-08-28

## Problem

Workspace Logs identified actor/tool/status but often could not answer the operator's primary question: what concrete action was performed. Relay activity intentionally excluded raw arguments, leaving terminal calls such as `terminal_exec` with only a generic target/result.

## Goal

Record and present a bounded, credential-redacted action summary while preserving the existing confidentiality boundary: no raw arguments, stdout/stderr, prompt, auth, environment, or unrestricted result payload enters the activity event.

## Implementation

- Extend `activity.event.v1` presentation metadata with optional `action` (max 256 chars).
- Derive tool-specific safe summaries at the relay execution boundary: terminal command/args, file path/range or edit count, bounded search scope, Git operation/ref, and HTTP method + host/path.
- Normalize workspace absolute paths and deterministically redact credential-shaped values before journaling/export.
- Preserve generic result confidentiality; terminal completion only extracts bounded exit code from the already-sanitized tool envelope.
- Persist/map the action through Nuxt ingest/read model and expose it in shared API types.
- Make Workspace Logs action-first: “What happened” and result are primary; actor/tool/evidence move to technical details. Historical rows without action retain a clear fallback.

## Verification

- Focused Rust activity test proves useful terminal action text and credential redaction.
- Web activity contract test covers `action` ingestion/mapping/UI.
- `pnpm guardrail` passes across web + Rust.
- Production Nuxt build passes and is deployed; app health returns HTTP 200.
- Release `ai-tools` build passes and the new binary is atomically installed at the operator binary path without restarting, signaling, or modifying the systemd relay service. The currently running relay process is intentionally left untouched; it will begin emitting the new action field on its next operator-controlled process start.
