# Architecture

AI Code has two primary runtime components and several external dependencies.

```text
Browser
  |
  | HTTPS / session cookie
  v
Nuxt / Nitro application
  |-- PostgreSQL               users, workspaces, conversations, providers, MCP config
  |-- model providers          AI generation
  |-- SMTP / Google / GitHub   application login options
  |-- Loki / Jaeger            optional telemetry
  |
  | MCP client
  v
Public MCP resource  <---------------- external MCP client / other MCP clients
  |
  | HTTPS edge / outbound tunnel
  v
127.0.0.1:47821
ai-tools relay
  |
  | OAuth resource-server validation + Bubblewrap
  v
owner-scoped coding filesystem / subprocesses
```

## Web application boundary

The Nuxt application owns:

- user sessions and application accounts;
- email/password verification and reset flows;
- optional Google/GitHub OAuth login;
- workspaces and conversations;
- model providers and encrypted provider credentials;
- MCP server configuration and tool orchestration;
- telemetry emitted by the web/server application.

The server follows a layered structure:

```text
server/api
  -> server/application
      <- server/infrastructure
```

HTTP routes compose and adapt requests, application modules own use-case semantics/contracts, and infrastructure modules implement database/provider/MCP/network integrations.

## Native execution boundary

`ai-tools relay` is the authority for coding-machine execution. The web server must not grow an alternate direct shell path.

The relay:

- implements MCP `2026-07-28` over Streamable HTTP (`POST /mcp`);
- binds to loopback rather than a public interface;
- refuses UID 0/root;
- uses Bubblewrap on Linux for filesystem/process containment;
- validates server-side tool authorization independently of client UI;
- supports both local and OAuth-protected remote modes;
- owns terminal timeout, cancellation, job concurrency, bounded output retention, and process-tree cleanup;
- exposes bounded native workspace inspection/search/read/edit/write/patch operations without routing routine filesystem work through a shell;
- exposes fixed-subcommand Git status/diff/log/show/blame reads with user/repository executable Git helpers neutralized.
- uses one component-aware protected credential-path policy for native workspace operations and Bubblewrap masking (`.ssh`, cloud/Docker/Kubernetes configuration, and common package-manager credential files); `.env.example` is intentionally not covered by this policy;
- treats terminal network access as an explicit operator capability (`RELAY_ALLOW_TERMINAL_NETWORK`), isolated by default, while dedicated HTTP/search tools remain separately classified network capabilities.

For the single-owner profile, `--execution-root` may be the canonical non-root home directory while `--dir` points at a particular starting repository. A request may change `cwd` between directories under the execution root without reconnecting the MCP client. Workspace path resolution canonicalizes existing read targets and validates write parents against this boundary. Recursive native traversal does not follow symlink directories; edit/write operations additionally use no-follow directory/file descriptors and same-directory atomic replacement semantics so validation-time containment is not treated as sufficient mutation safety.

## Remote MCP and OAuth

The remote relay is an **OAuth Resource Server**, not an Authorization Server.

The external Authorization Server—currently Keycloak in the reference deployment—owns login, authorization codes, PKCE, token issuance, client registration, signing keys, and identity lifecycle. The relay only validates the presented access token:

- issuer (`iss`);
- resource/audience (`aud`);
- signature through asymmetric JWKS;
- expiry/time validity;
- single allowed owner subject (`sub`);
- required `relay.coding` scope.

The canonical production pattern is:

```text
Internet
  -> public HTTPS hostname
  -> outbound-established tunnel/edge
  -> http://127.0.0.1:47821
  -> ai-tools relay --mode remote
```

Do not expose port `47821` through router/NAT forwarding and do not bind the relay to `0.0.0.0`.

## external MCP client and hosted Nuxt

external MCP client and the hosted Nuxt application can consume the same public MCP resource, but they authenticate independently.

- **external MCP client** performs its own interactive OAuth flow against the Authorization Server.
- **Hosted Nuxt** currently uses a private server-side access token only for the configured first-party MCP URL and only for the configured AI Code owner user ID.

Those two owner identities are intentionally different concepts: the Nuxt owner is an `ai_code.users.id`; the relay owner is the Authorization Server's stable `sub` claim.

## Capability policy

The first-party application classifies tool calls using structured effects (workspace read/write/delete, Git read, process execution, network, external mutation, and privileged bridges). Approval precedence is deny > ask > allow > default. MCP annotations remain client-facing hints; they do not replace relay enforcement. Argument-aware assessment treats shell/interpreter indirection and unknown wrappers as opaque, so a remembered tool-level approval cannot silently authorize them.

The first-party chat UI presents those same capability facts through category-driven tool cards and bounded approval summaries rather than raw tool JSON. Diff/result previews are capped, subagent/background state is compact, and hidden reasoning is never required for progress UX. Action observability reuses the Plan-035 OpenTelemetry/logger pipeline and its single allowlist sanitizer; semantic attributes may include stable tool/category/effect/policy/result/agent state, but never tool arguments/results, prompts, credentials, private absolute paths, or raw provider/tool errors. The relay remains the hard authority even when the UI shows an approval or risk classification.

## Tailscale boundary

The coding relay does not expose the host Tailscale local API socket by default. A local operator may opt in with `RELAY_ALLOW_TAILSCALE=true`, which bind-mounts only `RELAY_TAILSCALE_SOCKET` (default `/var/run/tailscale/tailscaled.sock`) into the Bubblewrap sandbox so commands such as `tailscale ip -4` can query the host daemon.

## Docker boundary

The coding relay does **not** expose the host Docker socket by default. For trusted single-owner local development, an operator may explicitly opt in with `RELAY_ALLOW_DOCKER=true`; that escape hatch permits the `docker` CLI and bind-mounts the daemon socket selected by `RELAY_DOCKER_SOCKET` (default `/var/run/docker.sock`) into the Bubblewrap sandbox. Docker daemon access is effectively host-level authority and therefore weakens the filesystem boundary. Production/remote deployments should keep it disabled unless the operator deliberately accepts that trust expansion.

## Internal module ownership and maintainability

The runtime trust boundaries above are unchanged by the maintainability refactor. Internally, the Rust relay now keeps stable crate facades while grouping implementation by responsibility:

- `application::execution` owns job lifecycle/result state and delegates process execution, request translation, and Bubblewrap construction to cohesive submodules;
- `application::workspace` owns the workspace capability facade and delegates secure path/no-follow primitives, listing, searching, reading, and atomic mutation to focused submodules;
- infrastructure transport keeps router/bootstrap composition separate from access-policy/OAuth orchestration and MCP request/tool/task handlers;
- the MCP interface keeps protocol/result types separate from the canonical tool catalog and schemas;
- core configuration keeps validated server configuration separate from the CLI declaration surface.

These splits are responsibility boundaries, not extension frameworks. Existing auth, sandbox, workspace-containment, cancellation, output-retention, Docker/Tailscale opt-in, and public MCP contracts remain authoritative in their existing policy owners.

Repository maintainability guardrails are enforced locally by `scripts/check-maintainability.mjs` through `pnpm verify:commit`: maintained production source has a hard 500-line threshold and cohesive implementation folders have a hard 15-direct-file threshold. Files above 400 lines and folders with 13–15 direct maintained files are review findings rather than automatic failures. Narrow exceptions require an exact path and concrete cohesion reason; broad/wildcard exceptions are rejected. The thresholds are signals for responsibility review, not permission to create meaningless helper files or wrapper folders.
