# Rust CLI Terminal Tool Process Safety

As part of the JS-to-Rust CLI migration (Plan 027, Step 4), this memory documents the required semantics for process lifecycle management and argument boundary preservation specifically applied to `terminal-tool`.

## 1. Timeout and Process Group Lifecycle
A critical invariant of `terminal-tool` is that processes must be deterministically terminated on timeout, leaving no uncontrolled descendants (zombie or orphaned children).

- **Insufficiency of default termination**: The standard Rust `.kill_on_drop(true)` configuration is insufficient on its own because it only sends `SIGKILL` to the immediate child process. If the shell command invoked other subprocesses, those descendants would be orphaned and left running in the background.
- **Process Group Isolation**: To ensure complete cleanup, the `terminal-tool` configures the spawned child with `cmd.process_group(0)` (on Unix platforms). This spawns the child into a new process group.
- **Group Signalling**: When the internal `--timeout` (e.g. 30000ms default) limit is reached, the tool handles the timeout by executing a native `libc::kill(-pid, SIGKILL)` call to signal the entire process group, effectively sweeping away all descendants deterministically.

## 2. Argument Boundary Preservation
To prevent unintended shell evaluation and injection vulnerabilities, `terminal-tool` leverages strict argument vector passing:
- Argument strings that contain spaces, empty strings `""`, or leading dashes `-` are passed seamlessly as explicit boundary elements.
- Shell metacharacters (such as `&`, `|`, `*`, `>`) are treated entirely as literal arguments to the command being invoked.
- Rust's standard library inherently escapes these arrays securely without invoking `/bin/sh -c` behavior under the hood.

## 3. Coverage and Evidence
These safety semantics are explicitly locked in by a robust integration test suite (`tests/terminal_tool_tests.rs`).
- Descendant testing creates a nested series of `sleep` jobs and asserts via `pgrep` that zero instances survive the timeout phase.
- Re-introduces the `--timeout` parameter contract that existed in the legacy JS implementation, ensuring application integrations still function accurately.
