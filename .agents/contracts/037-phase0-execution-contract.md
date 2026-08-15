# Plan 037 Phase 0 — execution contract

Status: audited before implementation on 2026-08-16.

## Protocol target

The relay continues to speak MCP `2026-07-28` Streamable HTTP. The current
Tasks target is the `io.modelcontextprotocol/tasks` extension (SEP-2663), not
the older `2025-11-25` core Tasks API. The current extension is server-directed:
the client opts in per request through
`params._meta.io.modelcontextprotocol/clientCapabilities.extensions`, and a
server may return `resultType: "task"` for `tools/call`. The implemented
operations are `tasks/get`, `tasks/update`, and `tasks/cancel`; `tasks/list` is
not part of the current stateless extension. Streamable HTTP task operations
use `Mcp-Name: params.taskId`.

Sources reviewed:

- https://tasks.extensions.modelcontextprotocol.io/specification/draft/tasks
- https://modelcontextprotocol.io/seps/2663-tasks-extension
- https://blog.modelcontextprotocol.io/posts/2026-07-28/
- https://help.openai.com/en/articles/12584461-developer-mode-apps-and-full-mcp-connectors-in-chatgpt-beta

ChatGPT/OpenAI documentation describes custom MCP apps and write-action
approval, but does not promise a terminal-style rendering of arbitrary task
progress or stdout chunks. The relay therefore exposes truthful protocol
state; client rendering remains a separate acceptance claim.

## Current implementation audit

- `terminal_exec`, `http_fetch`, and `web_search` all used an unconditional
  `300000` ms application/schema ceiling.
- Relay execution held the request's semaphore permit and HTTP request open
  until process exit.
- Relay output used bounded `read_to_end`; the terminal CLI used
  `wait_with_output` and a second 20 KB truncation policy.
- `timeout_ms = 0` became an immediate timeout in the terminal CLI.
- Output overflow killed the process instead of continuing to drain pipes.
- The configured directory and execution root were commonly the same project
  path. Plan 037 changes the recommended root to the canonical non-root owner
  home while retaining canonical containment and system-root rejection.

## Frozen implementation contract

- Terminal default timeout: 30 seconds.
- Terminal `timeout_ms = 0`: no deadline unless an explicit operator maximum
  is configured; a positive value is checked against that maximum.
- HTTP/search retain independent 30-second request safeguards and the existing
  request-level validation.
- Jobs are in-memory, bounded, owner/auth-context scoped, and expire after a
  one-hour completed-job TTL by default. Relay restart loses active jobs after
  shutdown cleanup.
- Maximum running jobs remains 16 by default; the permit belongs to the
  running job, not the creating request.
- Retained stdout/stderr is a bounded per-job tail (1 MiB total by default),
  with omitted-byte accounting and stream identity preserved.
- `terminal_exec` remains synchronous unless a client opts into the current
  Tasks extension. Non-Tasks clients use `terminal_job_start/get/cancel`.
- The job manager is the sole relay process lifecycle: it owns timeout,
  continuous pipe draining, bounded retention, process-group cancellation,
  and reap. The standalone terminal CLI no longer owns a competing timeout or
  output buffer.
- The execution root is the canonical owner home for the operator deployment;
  `cwd` is independently selected beneath that root. User toolchains are
  available only through explicit `--toolchain-path` allowlisted directories.
  Known credential directories/files are masked in the Bubblewrap namespace;
  this policy is re-proven by the deferred Phase 9 reviewer.
