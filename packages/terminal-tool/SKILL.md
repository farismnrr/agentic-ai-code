---
name: terminal-tool
description: LangChain and AI SDK terminal tool plus its native Rust CLI execution backend.
license: MIT
---

# @ai-code/terminal-tool

`@ai-code/terminal-tool` provides the TypeScript LangChain/AI SDK tool factory used by the application. The standalone command-line executable is no longer a JavaScript/npm `bin`; it is the native Rust unified `ai-tools` binary in [`../rust-tools/`](../rust-tools/).

## TypeScript usage

```ts
import { createTerminalTool, createTerminalAiTool } from '@ai-code/terminal-tool'

const myTerminalTool = createTerminalTool({
  cwd: '/my/workspace',
  assertSafeCommand: async (command) => {
    if (command === 'rm') throw new Error('Blocked by application policy')
  }
})

const result = await myTerminalTool.invoke({ command: 'ls', args: ['-la'] })
```

The injected guard is application policy. Do not infer the relay-agent security model from this standalone factory; relay execution has additional server-side authorization and Bubblewrap containment.

## Native CLI

Build all native tools from the repository root:

```bash
pnpm build:tools
```

For development, invoke the current Rust CLI directly:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin ai-tools -- terminal \
  --cwd /path/to/workspace \
  --allow-command ls \
  ls -la
```

The CLI accepts:

- `--cwd <path>` — working directory;
- `--allow-command <cmd>` — guarded executable allow entry; may be repeated;
- `--timeout <ms>` — execution timeout;
- `--no-guard` — explicit local bypass of the CLI guard.

`--no-guard` is not permission to weaken relay-agent authorization or sandboxing. The relay owns its own execution policy.

Use the binary help as the authoritative CLI reference:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin ai-tools -- terminal --help
```

Do **not** document or rely on `npx @ai-code/terminal-tool ...`; the package no longer exposes an npm CLI bin mapping.
