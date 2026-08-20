# Plan 050 — Propagate bounded coding-agent results through MCP

Status: CLOSED / VERIFIED (2026-08-20)

## Objective

Make successful `agent_delegate` calls useful to an orchestrating MCP client by
returning the completed provider's bounded final stdout alongside the existing
execution metadata, while preserving credential redaction and output limits.

## Root cause

The provider subprocess output was already captured in the execution
`OutputBuffer` and retained in `JobSnapshot`, but the agent-specific MCP result
serialized only `AgentResult` metadata. The synchronous `agent_delegate` path
returned that metadata result and therefore discarded the provider's answer at
the final application boundary.

## Scope

- Return successful provider stdout as `output` in the agent result JSON.
- Redact credential-shaped values before truncation.
- Bound the public provider result to 64 KiB and expose truncation/redaction
  indicators only when applicable.
- Keep failed-attempt logs and hidden provider reasoning out of the structured
  result; the field represents provider stdout/final output, not chain of
  thought.
- Extend the existing Plan 046 runtime acceptance with output propagation and
  secret-redaction assertions.

## Validation

- Plan 046 delegation acceptance, including the MCP application task path.
- Plan 048 capability discovery acceptance.
- Full repository commit gate.
- Release build, user-service restart, and live MCP `agent_delegate` smoke that
  asserts the response contains provider output and no secret-shaped value.

## Closure evidence

- `bash scripts/verify-046-agent-delegation.sh`: PASS.
- `bash scripts/verify-048-capability-discovery.sh`: PASS.
- `pnpm verify:commit`: PASS.
- `pnpm --filter @ai-code/rust-tools build`: PASS.
- Deployed `ai-tools 0.0.12` at `/home/farismnrr/.local/bin/ai-tools` matches
  `target/release/ai-tools` and the running service executable at
  `ec4a2a0a39d7d9bf35e40d703fa8a1ddad2e4caf74bb5de409a46c5e5d01f4ff`.
- `AI_TOOLS_BIN=/home/farismnrr/.local/bin/ai-tools
  scripts/verify-050-agent-result-mcp.sh`: PASS. This uses the real MCP HTTP
  `tools/list` and `tools/call` path against the deployed release binary and
  proves provider output propagation, redaction, no workspace change, and
  capability-filtered `codex` discovery.
- `REMOTE_MCP_URL=https://mcp.farismunir.my.id/mcp
  scripts/phase36-public-mcp-smoke.sh`: public edge and OAuth challenge PASS;
  authenticated remote discovery/delegation remains unavailable because no
  owner OAuth token was supplied. No token was fabricated or read from an
  unrelated credential source.
- Implementation commit: `5c31d71bbf9006053bcaf340fbbb2763da4c39c8`; after the
  closure-only documentation commit, local `fix/host-github-auth`, its
  configured `origin` tracking ref, and `git ls-remote` all match the final
  pushed HEAD.
