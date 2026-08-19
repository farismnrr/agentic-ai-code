# Getting started

This page gets the web application and native tools running from source. Remote MCP/ChatGPT setup comes later.

## 1. Prerequisites

Install:

- Node.js 22 or newer;
- pnpm **11.18.0** (the repository pins it through `packageManager`);
- Rust **1.95.0** for repository development;
- PostgreSQL;
- Git;
- Linux + `bwrap`/Bubblewrap if you plan to run `ai-tools relay`.

Optional integrations have additional requirements:

- SMTP for email verification/password reset;
- Google/GitHub OAuth credentials for those application login methods;
- Jaeger/Loki-compatible endpoints for telemetry;
- Keycloak or another compatible OAuth/OIDC Authorization Server for remote MCP;
- an outbound HTTPS tunnel such as Cloudflare Tunnel for public MCP access.

## 2. Clone and install

```bash
git clone https://github.com/farismnrr/ai-code.git
cd ai-code
pnpm install
```

`pnpm install` also:

- builds the Rust tools;
- prepares generated Nuxt files;
- installs the tracked Git pre-commit hook through `core.hooksPath=.githooks`.

## 3. Create local configuration

```bash
cp .env.example .env
```

At minimum for a normal local app setup, configure:

```dotenv
NUXT_PUBLIC_SITE_URL=http://localhost:3333
NUXT_DATABASE_URL=postgres://USER:PASSWORD@HOST:5432/ai-code
NUXT_SESSION_PASSWORD=<at-least-32-characters>
NUXT_MODEL_PROVIDER_SECRET_KEY=<32-byte-hex-key>
NUXT_WORKSPACES_ROOT=/absolute/path/to/your/workspaces
```

Generate the provider-encryption key with:

```bash
openssl rand -hex 32
```

Generate a strong session password with a password manager or, for example:

```bash
openssl rand -hex 32
```

Read [configuration.md](configuration.md) before enabling external services.

## 4. Prepare PostgreSQL

Create the database named by `NUXT_DATABASE_URL`, then apply repository migrations:

```bash
pnpm db:migrate
```

When schema changes are intentionally introduced during development, generate migrations with:

```bash
pnpm db:generate
```

Do not run `db:generate` merely as an installation step; committed migrations are the installation input.

## 5. Start the application

Development server:

```bash
pnpm dev
```

Default URL:

```text
http://localhost:3333
```

For final local runtime verification, prefer a clean production build instead of relying on a long-lived dev watcher:

```bash
pnpm build
pnpm preview
```

## 6. Configure a model provider

Create/login to an AI Code account, then use the application settings to add a supported provider/model. Provider API keys and secret custom-header values are encrypted at rest using `NUXT_MODEL_PROVIDER_SECRET_KEY`.

The application also supports a router endpoint through `NUXT_ROUTER_BASE_URL` / `NUXT_ROUTER_API_KEY` when that deployment model is desired.

## 7. Optional: run a local relay

Install Bubblewrap, then from the repository root:

```bash
./target/release/ai-tools relay \
  --mode local \
  --dir "$PWD" \
  --execution-root "$HOME" \
  --origin http://localhost:3333 \
  --allowed-host mcp.example.com
```

The relay remains loopback-only. `--execution-root` is the hard maximum filesystem boundary; `--dir` is the primary authorized workspace. Additional projects beneath that execution boundary must be authorized explicitly with `workspace_add`, can be inspected with `workspace_list` / `workspace_get`, and can be revoked with `workspace_remove`. Setting both values to `$HOME` intentionally authorizes the whole home tree and therefore removes most of the value of explicit workspace allowlisting.

For routine repository work, prefer the relay's native workspace tools: `directory_list` for structure, `file_search` for filename/glob discovery, `text_search` for source occurrences, `file_read` for bounded text ranges, `file_edit` for exact guarded replacement, and `file_write` for explicit create/full replacement. Use native Git tools for inspection plus bounded branch/stage/commit/merge/rebase/conflict workflows, native remote-Git tools for validated fetch/push/remote-branch operations, forge-neutral `change_request_*` tools for GitHub PR lifecycle work, and `apply_patch` for bounded multi-hunk source changes. Use `terminal_exec` for builds, tests, package managers, interpreters, project scripts, and operations that still do not have a native contract; ordinary terminal execution remains credential-isolated and is not the GitHub delivery bridge. Its `args` are direct child-process argv values, so flags beginning with `-` or `--` are valid and should be passed explicitly (for example `command="cargo", args=["--help"]` or `args=["check", "--locked"]`).

When an operator configures an approved LSP server, prefer the bounded `code_symbols`, `code_definition`, `code_references`, `code_implementations`, `code_hover`, `code_diagnostics`, and `code_rename_preview` tools over shell-driven source introspection. TypeScript/Vue support is opt-in, for example `--lsp-server typescript=typescript-language-server --lsp-server vue=vue-language-server` (or `RELAY_LSP_SERVER=typescript=typescript-language-server,vue=vue-language-server`) with those executables available through the reviewed safe PATH/toolchain paths. Without that registration, TypeScript/Vue `code_*` calls truthfully return `unsupported_language`. The relay also exposes bounded read-only repository resources for manifest/agent-guidance/status/HEAD context; those resources are server-owned views, not arbitrary file reads.

Workspace paths must remain both beneath the hard `--execution-root` boundary and inside the primary or explicitly authorized workspace roots. Relative paths use the selected workspace/cwd; absolute paths are accepted only when they remain authorized. Read-style operations may follow a symlink only when its resolved target stays inside an authorized root. Mutation tools fail closed on final symlinks and symlinked mutation parents rather than writing through them.

The relay always permits `localhost:<port>` and `127.0.0.1:<port>`. If the MCP client reaches the loopback listener using an externally-addressed Host, add that exact hostname with `--allowed-host` (or `RELAY_ALLOWED_HOSTS`). An entry without a port matches only that hostname without a port; configure `hostname:<port>` when a port must be allowed.

If your developer toolchains live outside the fixed system PATH, add reviewed user-owned directories explicitly with repeated `--toolchain-path` arguments or `RELAY_TOOLCHAIN_PATH`; do not inherit the entire shell PATH. For a general coding relay this commonly includes `$HOME/.cargo/bin` (Cargo/Rust), `$HOME/.bun/bin` (Bun), and the active fnm Node installation `bin` directory (Node/npm/pnpm/Corepack). Python, Go, Git, compilers, and build tools installed in the relay's fixed system PATH need no extra entry.

After changing relay access, socket, or toolchain configuration, rebuild/restart the relay and verify capabilities through the MCP client itself. A useful smoke test covers a simple command plus the configured Node/package manager, Rust, Tailscale, and Docker commands; host-shell success alone does not prove the Bubblewrap execution environment can reach them.

For local development that needs the host Tailscale daemon, add `--allow-tailscale` (or set `RELAY_ALLOW_TAILSCALE=true`). The socket defaults to `/var/run/tailscale/tailscaled.sock`; override it with `--tailscale-socket` or `RELAY_TAILSCALE_SOCKET` when needed.

For trusted local debugging that needs Docker, add `--allow-docker` (or set `RELAY_ALLOW_DOCKER=true`). For a non-default/rootless daemon, set `--docker-socket <absolute-path>` or `RELAY_DOCKER_SOCKET`. This explicitly exposes the selected host Docker daemon socket to terminal commands and therefore grants substantially more authority than the default sandbox.

## Next step

For public/remote tool access, continue with:

1. [Authentication](authentication.md)
2. [Keycloak](keycloak.md)
3. [Remote MCP deployment](remote-mcp.md)
4. [Connect ChatGPT](chatgpt.md)
