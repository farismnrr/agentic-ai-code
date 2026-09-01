# Tooling

## Environment and runtime config

Copy [`.env.example`](../../.env.example) → `.env` (gitignored) on a fresh clone. **`.env.example` is the environment-key inventory/source of truth**; keep it aligned with `nuxt.config.ts`/runtime consumers when configuration changes instead of maintaining a second exhaustive key list here.

Human/operator setup is documented under [`../../docs/`](../../docs/README.md); keep this file focused on agent-facing implementation/tooling invariants rather than duplicating the operator handbook.

Current configuration groups include dev server/public site URL, router/model-provider credentials, workspace root, PostgreSQL, session sealing, SMTP/optional OAuth providers, and OpenTelemetry/Jaeger/Loki.

Production runtime separation is mandatory: Docker is for the Nuxt
application only. Do not add the Rust workspace, relay binary, native build
targets, or native-tool adapter packages to the Nuxt image or mount them into
it. Build/install the Rust relay separately and keep it under its systemd
service. Nuxt-to-relay server work uses the first-party MCP URL and private
OAuth access-token configuration described in `NUXT_REMOTE_MCP_*`.

Workspace activity adds a separate configuration group: Nuxt uses
`NUXT_ACTIVITY_PAYLOAD_SECRET` and bounded `NUXT_ACTIVITY_RETENTION_DAYS`,
while the Rust relay uses `RELAY_ACTIVITY_MODE`, an owner-only state directory,
an HTTPS sink URL, a one-time source token, and bounded spool/ack-retention
limits. Keep the relay state directory outside workspace roots. Required mode
fails closed before execution when its local journal cannot admit a start; sink
outage after admission is asynchronous and recoverable.

Not every key is required for every workflow. Fill the values needed by the subsystem you are running; never commit secrets or real credentials to Markdown, plans, memories, fixtures, or examples.

### Stable conventions

- `NUXT_PORT` — dev port. Defaults to **3333** via `devServer.port` in `nuxt.config.ts`.
- `NUXT_HOST` — leave unset for the safe localhost-only default. When intentionally exposing dev to another device, bind to a specific trusted interface rather than `0.0.0.0`.
- `NUXT_PUBLIC_SITE_URL` — public runtime config; browser-visible by definition.
- `NUXT_WORKSPACES_ROOT` — operator-owned workspace filesystem boundary for the Nuxt application. Do not silently fall back to unrestricted filesystem browsing.

Nuxt runtime config binding is by convention: `NUXT_FOO_BAR` → `runtimeConfig.fooBar`, `NUXT_PUBLIC_FOO` → `runtimeConfig.public.foo`. Prefer `useRuntimeConfig()`/Nuxt config surfaces in application code instead of ad-hoc `process.env` reads.

The Rust `relay-agent` has its own CLI/environment contract under [`../../packages/relay-agent/SKILL.md`](../../packages/relay-agent/SKILL.md). Do not assume Nuxt runtime config and relay process config are interchangeable.

### Masih Awam MCP local coding relay

When an agent is operating through the local Masih Awam MCP relay, treat the relay as a deliberately constrained coding environment rather than a copy of the interactive login shell:

- The relay binds loopback, but an MCP bridge may preserve an external HTTP `Host` such as `mcp.example.com`. Keep `localhost:<port>` and `127.0.0.1:<port>` implicit, and add only the exact bridge authority with `--allowed-host` / `RELAY_ALLOWED_HOSTS`. Never disable Host validation or trust `X-Forwarded-Host`.
- Non-browser MCP clients may omit `Origin`. A missing Origin is valid only when the relay has a configured browser Origin policy; when an Origin is present it must still match the configured `--origin` exactly.
- The relay does not inherit the operator's full `$PATH`. System tools under the fixed safe PATH are available automatically; user-owned runtimes must be exposed explicitly with repeated `--toolchain-path` / `RELAY_TOOLCHAIN_PATH`. Explicit toolchain directories are prepended to the safe PATH so operator-selected runtimes take precedence over default entries. Typical coding directories are `$HOME/.cargo/bin`, `$HOME/.bun/bin`, and the active fnm Node installation `bin` directory. Prefer explicit reviewed directories over inheriting the login-shell PATH.
- Docker and Tailscale are separate authority expansions. Enable them only when the task needs them with `--allow-docker` and `--allow-tailscale`; the relay binds only their configured Unix sockets. Docker daemon access is substantially more privileged than ordinary sandboxed command execution.
- `terminal_exec` uses direct executable + argv semantics. Argument values are passed verbatim and may begin with `-` or `--`; prefer `command="cargo", args=["--help"]` or `command="cargo", args=["check", "--locked"]` rather than encoding child flags as relay options. The `command` executable itself must resolve from the relay safe PATH: do not use `./script.sh` or another executable path there; run repository scripts through an approved interpreter such as `command="bash", args=["scripts/check.sh"]`. Shell operators such as `&&`, `|`, redirects, glob expansion, and command substitution are not interpreted unless the agent explicitly invokes a shell such as `sh -lc` and doing so remains compatible with the execution policy.
- For a coding-capable relay, verify the actual runtime after restart instead of assuming configuration worked: a simple command, Node/package manager, Rust toolchain, Tailscale when enabled, and Docker when enabled.

A representative single-owner local coding profile is:

```sh
./target/release/ai-tools relay \
  --mode local \
  --port 47821 \
  --dir "$HOME" \
  --execution-root "$HOME" \
  --origin http://localhost:3333 \
  --allowed-host mcp.example.com \
  --allow-docker \
  --allow-tailscale \
  --toolchain-path "$HOME/.cargo/bin" \
  --toolchain-path "$HOME/.bun/bin" \
  --toolchain-path "<active-fnm-node-installation>/bin"
```

For a general coding relay, prefer `--dir "$HOME"` so the default working directory is neutral while task calls select a specific repository with `cwd`. Use `--dir "$PWD"` only for intentionally project-scoped relay instances. Keep deployment-specific hostnames and versioned runtime paths in operator configuration, not hardcoded in source or agent policy.

## Package manager and native toolchain

- Use **pnpm**; the exact pnpm version is pinned in root `package.json`.
- The native workspace is under `packages/rust-tools/`.
- Repository development pins **Rust 1.95.0**; `Cargo.toml` separately declares MSRV 1.88.0.
- `pnpm build:tools` builds the native binaries used by local tool/relay packages.
- This repository intentionally has **no CI**. Web tests live under top-level `test/`; Rust tests live under `packages/rust-tools/tests/`. Production files contain no inline test modules.

## Mandatory local commit gate

`pnpm install` runs [`../../scripts/install-git-hooks.sh`](../../scripts/install-git-hooks.sh), which makes [`.githooks/pre-commit`](../../.githooks/pre-commit) executable and configures local `core.hooksPath=.githooks`.

The hook executes the auto-scoped gate:

```sh
pnpm guardrail
```

For service-isolated verification, use `pnpm guardrail:nuxt` or
`pnpm guardrail:rust`. `pnpm guardrail:all` is reserved for a deliberate
cross-stack contract change. The explicit gates still run repository-wide
policy/architecture checks, but only the selected service's lint, typecheck,
tests, and test-layout scan.

That command always runs repository policy enforcement, agent-doc integrity, architecture checks, maintainability budgets, and test-layout policy. It then runs web or Rust lint/type/test commands only when that stack changed. If an applicable gate fails, the commit must not be created. Do not use `git commit --no-verify` or alter `core.hooksPath` to bypass it.

`scripts/check-test-layout.mjs` is part of the guardrail. It rejects test-like files outside the approved web/Cargo test locations and inline Rust/JavaScript test markers.

Plan numbers are not test architecture. Add feature-named tests under `test/` or Cargo `tests/`; do not add `verify-NNN`, `phase-NNN`, or composed per-plan validation scripts.

## Linting

Use `pnpm lint:web` for JavaScript/TypeScript/Vue and `pnpm lint:rust` for Rust. `pnpm lint` remains an explicit full-repository convenience command, not the default commit behavior.

`pnpm lint:fix` applies ESLint fixes and Rust formatting. Clippy findings still require deliberate code fixes.

## Type-checking

Use `pnpm typecheck:web` for Nuxt/Vue and `pnpm typecheck:rust` for the Cargo workspace. `pnpm typecheck` explicitly runs both when a full-repository check is actually desired.

The dedicated Nuxt prepare step generates the type project without coupling web verification to Rust or production bundling. Keep `pnpm build` as a separate runtime/bundling verification command when needed.

Do not simplify the type gate back to plain `nuxt typecheck`: this repository previously observed that wrapper exit successfully while real generated-project errors remained. The rationale and related Nuxt UI slot trap are preserved in the canonical [`../memories/README.md`](../memories/README.md#repository-policy-and-verification).

## Dependency/security verification

`pnpm audit` is not run by the pre-commit hook because it depends on registry/network state. It remains mandatory for dependency changes before merge. Security-sensitive Rust changes may additionally require `cargo audit` and focused Cargo tests.

### Maintainability checker

`node scripts/check-maintainability.mjs` is the authoritative repository-native source/file-folder budget check and is part of `pnpm guardrail`. It reports >400-line files and 13–15-file folders for review, fails unexplained >500-line files and >15-file folders, excludes generated/vendor/build/migration/evidence-style paths explicitly, and rejects wildcard/broad exceptions. `node scripts/check-maintainability.mjs --self-test` proves representative oversized-file and overfull-folder fixtures fail. Do not create a second threshold configuration elsewhere.
