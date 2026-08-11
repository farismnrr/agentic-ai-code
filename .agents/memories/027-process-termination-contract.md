# Process Termination Contract (027)

This document describes the cross-platform process termination contract for our Rust terminal-tool.

## Linux and macOS (Unix) Implementation
On Linux and macOS, process termination is handled by signaling the entire process group.
- A new process group is created when spawning the command using `cmd.process_group(0)`.
- When a timeout occurs, we send a `SIGKILL` to the process group using `libc::kill(-(pid as i32), libc::SIGKILL)`.
- This ensures that both the parent process and any child processes it spawned are terminated.

## Windows Support Status
- **Windows is NOT in the supported release matrix.** (As verified in `.github/workflows/ci.yml`, the build matrix only includes `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, and `aarch64-apple-darwin`).
- The current implementation is Linux/macOS only and Windows is explicitly unsupported.

## Platform Guards
The codebase uses conditional compilation (`cfg` attributes) to apply the appropriate process termination logic:
- `#[cfg(unix)]` guards are used to enable the process group creation (`cmd.process_group(0)`) and the process group kill logic (`libc::kill`).
- `#[cfg(not(unix))]` guards are used as a fallback (currently setting `cmd.kill_on_drop(true)`), but full robust termination on non-Unix platforms (like Windows Job Objects or TerminateProcess) is not implemented since they are outside our supported release matrix.
