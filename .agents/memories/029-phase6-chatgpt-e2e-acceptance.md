# Plan 029 Phase 6 acceptance evidence

This repository contains a deterministic acceptance harness at
`scripts/phase6-chatgpt-e2e.sh`. It verifies the local MCP contract and Plan
028 boundary hooks, and optionally probes protected-resource metadata when
`PHASE6_MCP_URL` points at a deployed relay.

## Evidence status

| Area | Repository check | Live evidence |
| --- | --- | --- |
| Tool discovery | tool catalog and required MCP headers are present | ChatGPT Scan Tools must be run in the configured workspace |
| OAuth/auth negatives | remote authorization, `relay.coding`, and insufficient-auth paths are present | invalid/expired/wrong-owner tokens require deployment credentials |
| Coding workflow | terminal, HTTP, search, and Plan 028 execution/security surfaces are present | inspect/edit/install/build/run/verify must be executed in ChatGPT |
| Plan 028 boundaries | workspace, symlink, privilege, Docker, and sandbox references are checked | boundary probes require a live authorized relay |

No live ChatGPT session, OAuth tenant, callback, or deployment URL is
available to this repository run. The harness therefore reports live evidence
as unavailable rather than claiming it passed. Phase 6 remains open until the
operator records screenshots/logs or equivalent redacted evidence for Scan
Tools, OAuth/refresh, coding workflow, and every negative boundary case.
