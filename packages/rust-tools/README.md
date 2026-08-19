# Rust Tools

This workspace contains the native implementation of the unified `ai-tools` binary, which provides:

- `terminal`
- `curl`
- `searxng`
- `relay`

The separate tool CLIs were migrated from JavaScript during historical Plan 027 and unified into a single binary during Plan 033. There is no supported JavaScript CLI fallback path.

## Toolchain

- **Edition:** Rust 2021
- **MSRV:** 1.88.0 (`Cargo.toml`)
- **Repository-pinned toolchain:** Rust 1.95.0 (`rust-toolchain.toml`)

Use the pinned toolchain for repository development/verification. The MSRV is a package compatibility floor, not the normal repository compiler.

## Architecture

The unified native executor (`ai-tools`) exposes the following subcommands:

- `terminal` — process execution with explicit guard/allow controls and timeout/process-group handling.
- `curl` — HTTP client with SSRF protections unless the explicit guard bypass is requested.
- `searxng` — SearXNG query client.
- `relay` — MCP `2026-07-28` server exposing controlled coding capabilities through the relay security boundary.

The `relay` subcommand executes other subcommands relative to its own executable rather than trusting arbitrary `$PATH`. The installation directory is therefore part of the trust boundary and must not be writable by the unprivileged runtime user.

### MCP coding tool surface

The relay exposes native workspace mutation/read tools, bounded Git read intelligence, and terminal/job/web tools:

- `directory_list(path=".", cwd?, depth=2, max_entries=100)` — deterministic bounded structure inspection; hard depth maximum 4 and returned-entry maximum 100; symlink directories are reported but not recursively followed.
- `file_search(pattern, cwd?, max_results=100)` — deterministic native glob discovery (`*`, `?`, and `**` path segments); hidden files are searchable while `.git`, `node_modules`, `target`, `.nuxt`, and `.output` directories are skipped; hard result maximum 100.
- `text_search(query, cwd?, glob?, regex=false, case_sensitive=true, max_results=50)` — ripgrep-backed source search through direct argv in a read-only sandbox; hard result maximum 100, 1 KiB match previews, and bounded serialized output.
- `file_read(path, cwd?, offset_line=1, limit_lines=200)` — strict UTF-8, 1-based line ranges; hard maximum 1,000 lines and 256 KiB returned text.
- `file_edit(path, old_text, new_text, cwd?, replace_all=false)` — exact UTF-8 replacement in an existing regular file; without `replace_all`, zero or multiple matches fail; target/update size is capped at 1 MiB and replacement text fields at 256 KiB.
- `file_write(path, content, cwd?, create_parents=false, overwrite=false)` — atomic create or explicit full replacement; content is capped at 1 MiB, new files use mode `0644`, and overwrite preserves the existing regular-file mode.
- `apply_patch(patch, cwd?, dry_run=false)` — constrained unified text patches for existing regular files; all files/hunks preflight before mutation, protected paths/symlinks/add-delete-rename/traversal/stale context fail closed, and per-file atomic replacement uses best-effort rollback if a later commit fails.
- `git_status`, `git_diff`, `git_log`, `git_show`, `git_blame`, `git_branch_*`, `git_stage`, `git_unstage`, `git_commit`, `git_merge_*`, `git_rebase_*`, `git_operation_status` — bounded local Git inspection/mutation with structured conflicts, protected-path filtering, repository-local identity, and executable Git config/helpers disabled.
- `git_remote_list`, `git_remote_branch_get`, `git_fetch`, `git_push`, `git_remote_branch_delete` — narrow GitHub remote transport with validated repository/ref identity, no force/arbitrary refspecs, isolated `gh auth git-credential`, and independent post-mutation verification.
- `change_request_list`, `change_request_get`, `change_request_create`, `change_request_update`, `change_request_checks`, `change_request_merge` — forge-neutral change-request contracts backed initially by a narrow GitHub `gh` adapter; arbitrary `gh api`, admin merge, auto-merge, implicit push/fork, and raw provider errors are not exposed.
- `issue_list`, `issue_get`, `issue_create`, `issue_update`, `issue_comment`, `issue_close`, `issue_reopen` — GitHub issue lifecycle operations with validated repository identity, bounded outputs, issue-only semantics (PRs fail closed), atomic comment support, duplicate close validation, verified post-state, and credential isolation through the privileged forge bridge.
- `workflow_list`, `workflow_get`, `workflow_run_list`, `workflow_run_get`, `workflow_run_jobs`, `workflow_job_log_preview` — read-only GitHub Actions observability with bounded run/job metadata and credential-redacted failed-log previews; repository identity is always derived from the validated Git remote.

Every workspace tool is scoped to the configured execution root. Relative paths resolve from optional `cwd`; contained absolute paths are permitted. Reads may follow only symlinks whose canonical targets stay contained. Recursive traversal does not follow symlink directories. Mutation paths use no-follow descriptor traversal, reject symlinked mutation parents/final targets, and use same-directory temporary files plus atomic commit semantics.

Prefer native workspace/Git/forge tools for structure, search, read, review, history, local Git mutation, remote synchronization, and change-request lifecycle work. Use `terminal_exec` for builds, tests, package managers, interpreters, repository scripts, and operations without a native contract; ordinary terminal execution is not the credential-bearing GitHub delivery bridge.

## Relay security/platform contract

- **Linux only.** Relay containment requires Bubblewrap (`bwrap`).
- **Unprivileged runtime.** The relay refuses UID 0.
- **Filesystem containment.** Execution is constrained to the configured execution root through Bubblewrap plus server policy.
- **Local/remote modes.** Local is loopback-oriented; remote is OAuth-protected and fail-closed.
- **Docker is opt-in.** The default sandbox does not expose the host Docker socket; trusted local development may enable the reviewed socket escape hatch explicitly, while remote/production deployments should normally leave it disabled.
- **Long-running / slow execution.** One bounded job manager owns spawn, output draining, timeout, cancellation, process-tree cleanup, retention, and concurrency for synchronous calls, MCP Tasks, and fallback jobs. `terminal_exec`, `web_search`, and read-like `http_fetch` methods (`GET`, `HEAD`, `OPTIONS`) may use that Tasks lifecycle when the client negotiates it; mutating HTTP methods stay synchronous until request-level idempotency/deduplication exists, and fast bounded native reads stay synchronous.
- **Timeout policy.** `timeout_ms = 0` is deadline-free unless an operator maximum is configured; terminal execution has no unconditional five-minute server ceiling. HTTP client round-trip deadlines remain separate from durable task execution lifetime.
- **Output policy.** stdout/stderr are drained continuously into bounded retained tails; exceeding retention omits older bytes instead of killing an otherwise valid process.

See [`../relay-agent/SKILL.md`](../relay-agent/SKILL.md), the canonical [memory](../../.agents/memories/README.md#rust-cli-migration-invariants), and [Plan 030 history](../../.agents/plans/030-previous-plans-summary.md) before changing these boundaries.

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

The repository has **no CI** and **no unit-test suite**. Rust quality is part of the mandatory local commit gate:

```bash
pnpm verify:commit
```

The root commands include:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked
```

Security-sensitive relay/MCP changes may additionally require `cargo audit`, `scripts/phase8-zero-bypass.sh`, and the deterministic acceptance scripts relevant to the current relay/MCP contract.

The old JavaScript parity harness is obsolete and is not a current verification source of truth.

## Release policy

There is **no automated GitHub Actions release workflow**. Native releases are a manual/operator action after local verification.

The supported relay release target remains **`x86_64-unknown-linux-gnu`** because production containment requires Linux + Bubblewrap. Do not document macOS/Windows relay support merely because simpler sibling CLI binaries may compile there.

When publishing native artifacts manually:

- build from the reviewed commit with the pinned Rust toolchain;
- run the mandatory local commit gate plus applicable Rust security checks;
- build the reviewed release bundle with `pnpm release:build vX.Y.Z`;
- publish the native archive and generated `SHA256SUMS` from the exact stable tag with `pnpm release:publish vX.Y.Z`;
- keep publish operations fail-closed to a clean `main` checkout whose requested tag points at `HEAD` and is already present on `origin`;
- do not weaken sandbox/platform contracts merely to broaden the release matrix.

The GitHub Release publishes the direct `ai-tools-x86_64-unknown-linux-gnu` asset required by the UI, a `ai-tools-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` archive, and `SHA256SUMS`. The same publish command builds and pushes the web image to GHCR for `linux/amd64` only. ARM64 is not part of the supported web-image release matrix unless a native/remote ARM64 builder is introduced and reviewed.

## CLI notes

Package-level TypeScript tool factories under sibling `packages/*/` are still application APIs, but the standalone executable CLI is the single Rust binary (`ai-tools`) in this workspace. Package skill docs must not advertise removed `npx @ai-code/*` bin mappings.

Use each binary's `--help` as the command-line source of truth.

## Internal ownership after Plan 039A

The crate boundaries are unchanged. Within them, large implementation surfaces are grouped by responsibility behind stable facades: application execution/workspace submodules, infrastructure transport access/MCP/tool handlers, interface protocol vs catalog/schema ownership, and core validated config vs CLI declaration. Do not collapse those responsibilities back into giant facade files when adding capabilities; extend the cohesive owner that already represents the reason to change.
