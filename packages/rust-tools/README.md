# Rust Tools

This package contains the native implementation of the unified `ai-tools` binary, which provides:

- `terminal`
- `curl`
- `searxng`
- `creative`
- `relay`

The separate tool CLIs were migrated from JavaScript during historical Plan 027 and unified into a single binary during Plan 033. There is no supported JavaScript CLI fallback path.

## Toolchain

- **Edition:** Rust 2021
- **MSRV:** declared in `Cargo.toml`
- **Repository-pinned toolchain:** declared in `rust-toolchain.toml`

Use the toolchain pinned by `rust-toolchain.toml` for repository development and verification. The MSRV declared in `Cargo.toml` is a package compatibility floor, not the normal repository compiler.

## Architecture

The unified native executor (`ai-tools`) exposes the following subcommands:

- `terminal` — process execution with explicit guard/allow controls and timeout/process-group handling.
- `curl` — HTTP client with SSRF protections unless the explicit guard bypass is requested.
- `searxng` — SearXNG query client.
- `creative` — foreground operator execution for reviewed Creative/Blender operations that are not safe to expose as <=60-second MCP calls; requests are supplied as JSON files and reuse the same project/job/graph/owner configuration and durable state.
- `relay` — MCP `2026-07-28` server exposing controlled coding capabilities through the relay security boundary.

The `relay` subcommand executes other subcommands relative to its own executable rather than trusting arbitrary `$PATH`. The installation directory is therefore part of the trust boundary and must not be writable by the unprivileged runtime user.

### Creative foreground operator execution

Long-running Creative/Blender work is not a public MCP execution surface. Prepare a reviewed JSON request file containing the same project/job/graph arguments, then run it in the foreground with the same reviewed `RELAY_*` environment used by the relay:

```bash
RELAY_ENABLE_CREATIVE=true ai-tools creative \
  --tool blender-render \
  --input /path/to/request.json
```

Supported operator-only tool selectors include `creative-job`, `creative-graph`, `blender-execute-python`, `blender-animation-preview`, `blender-render`, `blender-asset-import`, `blender-asset-export`, `blender-checkpoint-create`, and `blender-checkpoint-restore`. `creative-job` accepts only the internal `wait` execution action; `creative-graph` accepts only execute/rerun actions. The command intentionally has no background/detach mode and no 60-second wrapper; normal stdout/stderr remains operator-visible. It reuses the same workspace, owner, binding, budget, and durable Creative state rather than creating a second execution model.

### MCP coding tool surface

The relay exposes native workspace mutation/read tools, bounded Git read intelligence, and terminal/job/web tools:

- `directory_list(path=".", cwd?, depth=2, max_entries=100)` — deterministic bounded structure inspection; `depth=1` means direct children, hard depth maximum 4 and returned-entry maximum 100; symlink directories are reported but not recursively followed.
- `file_search(pattern, cwd?, max_results=100, exclude?)` — deterministic native glob discovery (`*`, `?`, and `**` path segments) with up to 16 exclusion globs; hidden files are searchable while dependency/generated trees such as `.git`, `node_modules`, `target`, `.pnpm-store`, `.nuxt`, `.output`, `dist`, `coverage`, `vendor`, and `.cache` are skipped; hard result maximum 100.
- `text_search(query, cwd?, glob?, regex=false, case_sensitive=true, max_results=50, exclude?)` — ripgrep-backed source search through direct argv in a read-only sandbox with up to 16 negative globs; hard result maximum 100, 1 KiB match previews, and bounded serialized output.
- `file_read(path, cwd?, offset_line=1, limit_lines=200)` — strict UTF-8, 1-based line ranges; hard maximum 1,000 lines and 256 KiB returned text; returns `next_offset_line` when truncated and a complete-file SHA-256 when the file is within the 1 MiB mutation ceiling.
- `file_read_multiple(paths, cwd?, offset_line=1, limit_lines=200)` — the same bounded range read for up to 16 files in one call, with per-file success/error isolation and a 512 KiB combined response ceiling.
- `file_edit(path, old_text, new_text, cwd?, replace_all=false, dry_run=false, expected_sha256?)` — exact UTF-8 replacement in an existing regular file; without `replace_all`, zero or multiple matches fail; `dry_run` validates without mutation and `expected_sha256` prevents stale writes; target/update size is capped at 1 MiB and replacement text fields at 256 KiB.
- `file_write(path, content, cwd?, create_parents=false, overwrite=false, expected_sha256?)` — atomic create or explicit full replacement; an overwrite may require the SHA-256 returned by `file_read` to reject stale content; content is capped at 1 MiB, new files use mode `0644`, and overwrite preserves the existing regular-file mode.
- `apply_patch(patch, cwd?, dry_run=false)` — constrained unified text patches for existing regular files; all files/hunks preflight before mutation, protected paths/symlinks/add-delete-rename/traversal/stale context fail closed, and per-file atomic replacement uses best-effort rollback if a later commit fails.
- `git_status`, `git_diff`, `git_log`, `git_show`, `git_blame`, `git_branch_*`, `git_stage`, `git_unstage`, `git_commit`, `git_merge_*`, `git_rebase_*`, `git_operation_status` — bounded local Git inspection/mutation with structured conflicts, protected-path filtering, repository-local identity, and executable Git config/helpers disabled.
- `git_remote_list`, `git_remote_branch_get`, `git_fetch`, `git_push`, `git_remote_branch_delete` — narrow GitHub remote transport with validated repository/ref identity, no force/arbitrary refspecs, isolated `gh auth git-credential`, and independent post-mutation verification.
- `change_request_list`, `change_request_get`, `change_request_create`, `change_request_update`, `change_request_checks`, `change_request_merge` — forge-neutral change-request contracts backed initially by a narrow GitHub `gh` adapter; arbitrary `gh api`, admin merge, auto-merge, implicit push/fork, and raw provider errors are not exposed.
- `issue_list`, `issue_get`, `issue_create`, `issue_update`, `issue_comment`, `issue_close`, `issue_reopen` — GitHub issue lifecycle operations with validated repository identity, bounded outputs, issue-only semantics (PRs fail closed), atomic comment support, duplicate close validation, verified post-state, and credential isolation through the privileged forge bridge.
- `workflow_list`, `workflow_get`, `workflow_run_list`, `workflow_run_get`, `workflow_run_jobs`, `workflow_job_log_preview` — read-only GitHub Actions observability with bounded run/job metadata and credential-redacted failed-log previews; repository identity is always derived from the validated Git remote.
- `dependabot_alert_list/get`, `code_scanning_alert_list/get`, `secret_scanning_alert_list/get/locations` — bounded read-only repository security visibility. Secret-scanning requests force `hide_secret=true` and public results unconditionally omit literal secret values and provider metadata.
- `workflow_dispatch`, `workflow_run_rerun`, `workflow_run_cancel` — narrow GitHub Actions mutations only; dispatch requires numeric workflow ID plus explicit ref and bounded string inputs, while rerun/cancel require numeric run IDs.

The structured workspace read/write/search tools (`directory_list`, `file_search`, `text_search`, `file_read`, `file_read_multiple`, `file_edit`, `file_write`, and `apply_patch`) advertise MCP `outputSchema` and return matching `structuredContent` as the single success payload. Their required MCP `content` array is empty on success instead of duplicating JSON into text. Successful structured results are validated against the advertised schema before the relay sends a success response.

Every workspace tool is scoped to the configured execution root. Relative paths resolve from optional `cwd`; contained absolute paths are permitted. Reads may follow only symlinks whose canonical targets stay contained. Recursive traversal does not follow symlink directories. Mutation paths use no-follow descriptor traversal, reject symlinked mutation parents/final targets, and use same-directory temporary files plus atomic commit semantics.

Prefer an active dedicated MCP tool for an operation it fully covers, including workspace inspection/editing, remote Git transport, SSH diagnostics, HTTP/web, forge/issues/workflows, alerts, and messaging. Use `terminal_exec` for builds, tests, package managers, interpreters, repository scripts, shell pipelines, and operations without an active structured contract; ordinary terminal execution is not the credential-bearing GitHub delivery bridge. The current runtime catalog is composed from one retained base plus explicitly enabled optional capabilities. That retained base intentionally omits local Git and LSP wrappers, so terminal is the supported path for those operations. Numbered catalog snapshots under `.agents/contracts/` are historical audit artifacts, not alternate active runtime versions.

## Relay security/platform contract

- **Linux only.** Relay containment requires Bubblewrap (`bwrap`).
- **Unprivileged runtime.** The relay refuses UID 0.
- **Filesystem containment.** Execution is constrained to the configured execution root through Bubblewrap plus server policy.
- **Local/remote modes.** Local is loopback-oriented; remote is OAuth-protected and fail-closed.
- **Explicit listener binding.** `--bind-host` / `RELAY_AGENT_BIND_HOST` defaults to
  `127.0.0.1`. Local mode remains loopback-only; a remote non-loopback bind requires
  OAuth and an exact configured `--origin` / `RELAY_AGENT_ORIGIN`.
- **Docker is read-only by default, full authority by explicit opt-in.** Arbitrary terminal processes do not receive the host Docker socket. A direct `docker` call may receive the configured socket only after semantic normalization through the bounded diagnostic allowlist. `RELAY_ALLOW_DOCKER=true` remains the explicit full-authority escape hatch for trusted single-owner development.
- **SSH diagnostics are a separate first-class MCP capability.** `RELAY_ALLOW_SSH=true` enables `ssh_readonly_exec`; it does not enable ordinary terminal networking. Clients provide a final configured alias, an optional bounded `via` alias chain, and a structured remote diagnostic command/args. The relay parses only a safe connectivity subset of operator SSH config (including contained `Include` files and alias-only `ProxyJump`) and then starts pinned OpenSSH with internal `-F /dev/null`, strict host-key verification, key-only `BatchMode`, no agent, no PTY, no user-selected forwarding, no connection multiplexing, no local commands, and null stdin. Multi-hop transport is relay-generated and the diagnostic command executes only on the final alias. Raw SSH flags/config/key paths are not part of the MCP schema. The SSH sandbox receives host networking plus read-only access to the exact resolved identity/known-host files for every hop, with the local workspace hidden entirely; it never receives the local Docker/Tailscale/agent sockets. Generic `terminal_exec` rejects direct `ssh`/`scp`/`sftp`, and its sandbox masks those client executables so the dedicated tool remains the canonical supported SSH path.
- **Remote SSH execution is fail-closed but broadly read-only.** Supported diagnostics are positive semantic policies, not arbitrary shell access. Bounded Docker logs/list/stats/top/safe inspect projections, validated `docker exec`/`docker compose exec -T` read-only nested commands, PostgreSQL/MySQL/MariaDB/SQLite/Redis inspection, selected host/file/Git diagnostics, bounded `journalctl`/read-only `systemctl`, and validated pipelines are allowed. Docker lifecycle mutations, interactive/detached/privileged exec, redirects/background jobs, shells/interpreters, privilege escalation, and unknown command families remain denied. Full secret-bearing `docker inspect`/Compose environment output is not exposed by default.
- **Database inspection requires least privilege.** Configure `RELAY_SSH_DB_READONLY_USER` for PostgreSQL/MySQL and `RELAY_SSH_READONLY_REDIS_USER` together with `RELAY_SSH_READONLY_REDIS_PASSWORD_FILE` for Redis. The Redis password file must be an owner-only regular file beneath `RELAY_SSH_ROOT`; its bounded contents are fed only over process stdin to `redis-cli --askpass`, never exposed through MCP arguments, process argv, activity, or logs. The relay combines dedicated read-only identities with engine-level read-only/session controls, client hardening, semantic query/command validation, statement/output bounds, and denial of expensive or ambiguous operations. It never falls back to owner/superuser/migration credentials. A requested remote mutation must be performed manually by the operator; the relay returns a policy denial instead of attempting it.
- **Long-running / slow execution.** One bounded job manager remains the internal process-lifecycle authority for spawn, output draining, timeout, cancellation-on-request-drop, process-tree cleanup, retention, and concurrency. Retained process-like coding tools are synchronous-only and do not expose caller-selected MCP Tasks execution. Work that cannot finish inside the 60-second public ceiling must be handed to the human operator as a foreground command rather than detached.
- **Timeout policy.** Public process-like tool runtimes are capped at 60 seconds. `terminal_exec`, `ssh_readonly_exec`, `http_fetch`, and `web_search` use a 30-second default where caller-selectable timeout is exposed; schemas reject values outside `1..=60000`. Operator terminal configuration may lower but never raise the terminal ceiling. Work expected to exceed it must be handed to the human operator as a foreground command without a timeout wrapper.
- **Output policy.** stdout/stderr are drained continuously into bounded retained tails; exceeding retention omits older bytes instead of killing an otherwise valid process.

See [`../relay-agent/SKILL.md`](../relay-agent/SKILL.md), the canonical [memory](../../.agents/memories/README.md#rustnative-tool-invariants), and [Plan 030 history](../../.agents/plans/030-previous-plans-summary.md) before changing these boundaries.

## Build

From repository root:

```bash
pnpm build:tools
```

Or directly:

```bash
cargo build --manifest-path Cargo.toml --release --locked --bin ai-tools
```

## Mandatory commit verification

The repository has **no hosted CI**. Rust integration tests live under `packages/rust-tools/tests/`, and Rust quality is part of the mandatory local commit gate:

```bash
pnpm guardrail
```

The root commands include:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked
```

Security-sensitive relay/MCP changes may additionally require `cargo audit` and focused Cargo tests/examples relevant to the affected contract. Repository-wide structural checks remain owned by `pnpm guardrail`; do not recreate plan-numbered verification scripts.

The old JavaScript parity harness is obsolete and is not a current verification source of truth.

## Release policy

There is **no automated GitHub Actions release workflow**. Native releases are
a manual/operator action after local verification. The native `ai-tools`
version is independent from the Nuxt web version and is published through a
GitHub Release containing only the CLI artifacts. The Nuxt application is
published separately as a Docker image to GHCR.

The unified `ai-tools` binary is packaged for these targets:

- `x86_64-unknown-linux-gnu` as a `.tar.gz` plus the direct binary;
- `aarch64-unknown-linux-gnu` as a `.tar.gz` plus the direct binary;
- `x86_64-apple-darwin` as a `.tar.gz` plus the direct binary;
- `aarch64-apple-darwin` as a `.tar.gz` plus the direct binary;
- `x86_64-pc-windows-gnu` as a `.zip` plus the `.exe` direct binary.

The `relay` subcommand remains Linux-only because production containment
requires Bubblewrap. macOS and Windows artifacts provide the portable CLI
surfaces; their existence does not imply non-Linux relay support.

When publishing native artifacts manually:

- build from the reviewed commit with the pinned Rust toolchain;
- install the requested Rust targets and `cargo-zigbuild` when the host is not
  one of those targets;
- run the mandatory local commit gate plus applicable Rust security checks;
- build the reviewed CLI bundle with `pnpm release:build vX.Y.Z[-beta]`;
- publish the generated archives, direct binaries, the canonical Masih Awam
  workspace-workflow Skill ZIP, metadata, and `SHA256SUMS` with
  `pnpm release:publish:cli vX.Y.Z[-beta]`;
- keep the CLI publisher fail-closed to a clean `main` checkout whose
  requested CLI tag points at `HEAD` and is already present on `origin`;
- do not weaken sandbox/platform contracts merely to broaden the release
  matrix.

The separate `pnpm release:publish:container vX.Y.Z[-beta]` command builds and
pushes the Nuxt web image to GHCR for `linux/amd64` and `linux/arm64`. Docker
Desktop on Windows and macOS runs this Linux-based image; native Windows and
macOS executables remain CLI release artifacts. Stable web images use
`vX.Y.Z`, `X.Y.Z`, and `latest`; beta images use the `vX.Y.Z[-beta]` and
`X.Y.Z[-beta]` version tags and do not move `latest`.

## CLI notes

Package-level TypeScript tool factories under sibling `packages/*/` are still application APIs, but the standalone executable CLI is the single Rust binary (`ai-tools`) in this workspace. Package skill docs must not advertise removed `npx @ai-code/*` bin mappings.

Use each binary's `--help` as the command-line source of truth.

## Internal ownership

The native implementation is intentionally one Cargo package. Large implementation surfaces remain grouped by responsibility behind stable module facades: `application` execution/workspace submodules, `infrastructure` transport access/MCP/tool handlers, `interfaces` protocol vs catalog/schema ownership, and `core` validated config vs CLI declaration. Do not collapse those responsibilities into giant facade files when adding capabilities; extend the cohesive owner that already represents the reason to change. `scripts/check-architecture.sh` replaces the former inter-crate dependency graph as the repository-enforced layer-direction guard.
