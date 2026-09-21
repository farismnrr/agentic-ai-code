# Getting started

Choose the path that matches what you are installing. The released `ai-tools`
CLI is portable across Linux, macOS, and Windows, while the production MCP
relay is **Linux + Bubblewrap only**. Building the full AI Code application
from source is a separate developer/operator workflow. Normal portable CLI
users do **not** need Node.js, pnpm, Rust, PostgreSQL, Bubblewrap, Zig,
`cargo-zigbuild`, or a macOS SDK.

## 1. Install the released `ai-tools` CLI

Download the current native release from the
[GitHub Releases page](https://github.com/farismnrr/agentic-ai-code/releases).

Download the artifact for your machine:

| Platform | Architecture | Recommended artifact | Relay support |
| --- | --- | --- | --- |
| Linux | x86_64 / amd64 | `ai-tools-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` | Yes, with Bubblewrap |
| Linux | ARM64 / aarch64 | `ai-tools-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz` | Yes, with Bubblewrap |
| macOS | Intel | `ai-tools-vX.Y.Z-x86_64-apple-darwin.tar.gz` | No |
| macOS | Apple Silicon | `ai-tools-vX.Y.Z-aarch64-apple-darwin.tar.gz` | No |
| Windows | x86_64 / amd64 | `ai-tools-vX.Y.Z-x86_64-pc-windows-gnu.zip` | No |

The release also includes direct binaries for each target, plus
`RELEASE-METADATA.json`, `SHA256SUMS`, and the workspace workflow Skill ZIP.
The archive is usually the easiest installation choice.

### Linux and macOS

Extract the archive, place `ai-tools` somewhere on your `PATH`, and verify it:

```bash
tar -xzf ai-tools-vX.Y.Z-<target>.tar.gz
mkdir -p "$HOME/.local/bin"
install -m 0755 ai-tools "$HOME/.local/bin/ai-tools"
ai-tools --version
```

Create `$HOME/.local/bin` first if needed and ensure it is on your shell
`PATH`. You may also use another user-owned executable directory.

### Windows PowerShell

Extract the ZIP and place `ai-tools.exe` in a directory on your user `PATH`.
For example:

```powershell
New-Item -ItemType Directory -Force "$HOME\bin" | Out-Null
Expand-Archive .\ai-tools-vX.Y.Z-x86_64-pc-windows-gnu.zip -DestinationPath "$HOME\bin" -Force
$env:Path = "$HOME\bin;$env:Path"
ai-tools.exe --version
```

Persist `$HOME\bin` in your user `PATH` if you want the command available in
future PowerShell sessions.

### Verify release checksums

Download `SHA256SUMS` beside the artifact. On Linux:

```bash
sha256sum --check SHA256SUMS
```

On macOS:

```bash
shasum -a 256 -c SHA256SUMS
```

On Windows PowerShell, compare the artifact hash with its entry in
`SHA256SUMS`:

```powershell
Get-FileHash .\ai-tools-vX.Y.Z-x86_64-pc-windows-gnu.zip -Algorithm SHA256
Get-Content .\SHA256SUMS
```

## 2. Install the Masih Awam workspace workflow Skill

The same release publishes
`masih-awam-workspace-workflow-vX.Y.Z.zip`. It is generated from the tracked
canonical source at
`.agents/skills/masih-awam-workspace-workflow/SKILL.md`.

For a client that supports Skill archive import, upload/import the ZIP unchanged.
If the client expects an unpacked Skill directory instead, extract it and keep
this layout intact:

```text
masih-awam-workspace-workflow/
└── SKILL.md
```

The Skill describes the workspace-agnostic Masih Awam development workflow:
resolve the active workspace, prefer available Masih Awam MCP capabilities,
respect repository-local guidance and authorization boundaries, and report only
verified work. Installing the Skill does not install the CLI or start a relay.

## 3. Linux only: run the relay

The `relay` subcommand is supported only on Linux because its production
security boundary requires Bubblewrap (`bwrap`). The macOS and Windows release
artifacts provide portable CLI surfaces only; they do **not** provide a
supported relay server.

Install Bubblewrap with your Linux distribution package manager, then configure
a workspace root and start the relay from the installed binary. A local example:

```bash
export RELAY_WORKSPACE_ROOT="$HOME/Documents/Projects"

ai-tools relay \
  --mode local \
  --origin http://localhost:3333
```

The relay refuses to run as root. Its default workspace/execution boundary is
`$HOME/Documents/Projects`; use an explicit narrower root when you intentionally
want a project-scoped relay. If a reverse proxy sends a different `Host` header,
allow that exact host with repeated `--allowed-host` flags or
`RELAY_ALLOWED_HOSTS`.

For a production or public MCP endpoint, do not stop at this example. Follow
[Remote MCP deployment](remote-mcp.md), then
[Connect an MCP client](mcp-client.md). Read
[Configuration](configuration.md) and [Security](security.md) before exposing
the service.

## 4. Develop or operate the full AI Code application from source

You need this workflow only when you are developing the repository or operating
the Nuxt application. A released portable `ai-tools` binary is independent of
the web application's Node.js, pnpm, Rust, and PostgreSQL setup.

### Prerequisites

Install:

- a Node.js version compatible with the repository's current dependencies;
- the pnpm version pinned by the repository `packageManager` field;
- the Rust toolchain pinned by `rust-toolchain.toml` for repository development;
- PostgreSQL;
- Git;
- Linux + Bubblewrap only if this checkout will also run the relay.

Then clone and install:

```bash
git clone https://github.com/farismnrr/agentic-ai-code.git
cd agentic-ai-code
pnpm install
```

`pnpm install` installs JavaScript dependencies, prepares Nuxt generated files,
and installs the tracked Git hooks. Build the native tools separately when
needed with `pnpm build:tools`.

Create local configuration:

```bash
cp .env.example .env
```

At minimum for a normal local application setup, configure:

```dotenv
NUXT_PUBLIC_SITE_URL=http://localhost:3333
NUXT_DATABASE_URL=postgres://USER:PASSWORD@HOST:5432/ai-code
NUXT_SESSION_PASSWORD=<at-least-32-characters>
NUXT_MODEL_PROVIDER_SECRET_KEY=<32-byte-hex-key>
NUXT_WORKSPACES_ROOT=/absolute/path/to/your/workspaces
```

Generate local secrets with a password manager or, for example:

```bash
openssl rand -hex 32
```

Create the database named by `NUXT_DATABASE_URL`, apply committed migrations,
and start development:

```bash
pnpm db:migrate
pnpm dev
```

The default local URL is `http://localhost:3333`. For final local runtime
verification use a fresh production build:

```bash
pnpm build
pnpm preview
```

Do not run `pnpm db:generate` as an installation step; that command is for
intentional schema development.

For contributor workflow and local validation, continue with
[Development](development.md).

## 5. Release maintainers

Cross-platform release-host setup is intentionally not part of normal
installation. Maintainers who build the five-target native release matrix need
additional Rust targets, Zig/cargo-zigbuild, and—when producing Apple targets
from a non-macOS host—an authorized macOS SDK through `SDKROOT`.

Those requirements and the release commands live only in
[Releases](releases.md).
