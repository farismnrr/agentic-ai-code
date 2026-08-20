# Plan 049 — Live relay host authentication and provider acceptance

Status: BLOCKED — authenticated remote MCP owner token unavailable

## Objective

Close the gap between the reviewed host-auth/delegation implementation and the
user-level systemd relay. Prove the narrow Bubblewrap mounts through the MCP
execution path, deploy the release binary selected by systemd, and record live
provider capability and delegation evidence without exposing credentials.

## Root-cause findings

- The running service had not reloaded/restarted after the drop-in changed
  AI_TOOLS_BIN; its process was still
  /home/farismnrr/.local/share/ai-code/bin/ai-tools.
- Host gh auth status was authenticated, but the relay unauthenticated local
  /mcp request was correctly rejected by the remote OAuth policy, so that
  request did not prove sandbox auth visibility.
- The repository source already contains the reviewed narrow host-auth mount,
  independent terminal/agent network flags, Codex 0.148.0 argv, and provider
  capability filtering. Existing Plan 046/048 deterministic gates pass.
- Codex 0.148.0 also needs transient state below `.codex` during a headless
  run. A plain read-only auth-root bind made the provider exit 1; Bubblewrap
  `--tmp-overlay` preserves the host session as a lower layer while discarding
  provider writes. Version-manager symlink paths likewise need namespace
  parent directories plus `--symlink` recreation so Node relative links keep
  resolving.
- A third installed provider's local status and diagnostic checks pass on this
  host, but its actual headless adapter returns exit 1 with an
  organization-level subscription-disabled message. The deployment therefore
  uses an explicit fail-closed allowlist containing only the two providers
  whose live smoke passed; the unavailable provider is not advertised.

## Acceptance

- scripts/verify-049-host-auth-sandbox.sh exercises actual Bubblewrap-backed
  MCP terminal, LSP, and hook paths: read-only/writable opt-in auth, default
  masking, non-terminal masking, and unrelated credential masking.
- Plan 046 delegation and Plan 048 capability acceptance pass.
- Release binary, systemd effective configuration, post-restart health, and
  artifact hash are recorded.
- The Plan 046 fixture now proves provider writes under an auth root are
  ephemeral and do not create a host-side file.
- Authenticated live MCP schema and provider smoke results are recorded when
  the current MCP connector supplies an owner token; absent external OAuth
  authority is reported as an exact unproven condition rather than a pass.
- Human deployment documentation and canonical memory remain synchronized.

## Closure evidence and external blocker

- The final deployed release hash is `db6e0cd248c2f86b742a4c42446de7059b6f72044e13fe51a7c3d23893466364`; systemd is active and its process executable matches `/home/farismnrr/.local/bin/ai-tools`.
- The live local MCP path proved authenticated `gh` status, a read-only GitHub repository query, Full catalog provider enum `[codex, agy]`, and successful delegation for the two allowed providers with no workspace change. The excluded provider is absent from the deployment allowlist after its real headless invocation was rejected by the provider organization entitlement.
- The public remote edge and OAuth challenge pass, but authenticated remote `server/discover`/`tools/list` and remote delegated smoke remain unproven because this session has no owner OAuth access token or connected authenticated MCP client. No token was fabricated or read from unrelated credentials.
- This external authorization gap is the sole remaining Definition-of-Done blocker; rerun the authenticated remote MCP acceptance after supplying the owner token through the approved MCP connector, then change this status only if those live checks pass.

## Closeout

Update this status and .agents/memories/README.md only with durable facts
confirmed by executed commands. Run the docs verifier and pnpm verify:commit,
then commit and push the task-owned repository changes on
fix/host-github-auth.
