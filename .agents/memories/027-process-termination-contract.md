# Process Termination Contract (027)

This document describes the process-termination contract established for the Rust `terminal-tool` during Plan 027.

## Unix implementation

On supported Unix execution paths, process termination is handled by signaling the entire process group.

- A new process group is created when spawning the command using `cmd.process_group(0)`.
- When a timeout occurs, the implementation sends `SIGKILL` to the process group using `libc::kill(-(pid as i32), libc::SIGKILL)`.
- This is intended to terminate both the direct child and descendants that remain in that process group.

## Windows/non-Unix status

Full Windows process-tree termination semantics such as Job Objects are not implemented by this contract. The non-Unix fallback is not equivalent to the Unix process-group guarantee.

Do not infer current platform/release support from an old CI matrix: the repository intentionally has no CI workflow now. Check current package/source and release guidance before promising a platform. In particular, the production `relay-agent` release contract is Linux + Bubblewrap even though this memory concerns the sibling `terminal-tool` process behavior.

## Platform guards

The codebase uses conditional compilation (`cfg` attributes) around platform-specific process behavior:

- `#[cfg(unix)]` enables process-group creation and group-kill logic.
- non-Unix paths use the implementation's available fallback behavior but do not provide the same robust descendant-termination guarantee documented above.

If platform behavior changes, verify the actual source and update this memory instead of reviving a historical CI matrix as the source of truth.
