---
name: curl-tool
description: LangChain HTTP tool plus its native Rust CLI with SSRF protection.
license: MIT
---

# @ai-code/curl-tool

`@ai-code/curl-tool` provides the TypeScript LangChain HTTP tool factory used by the application. The standalone executable is now the native Rust `curl-tool` binary in [`../rust-tools/`](../rust-tools/), not an npm `bin`.

## TypeScript usage

```ts
import { createCurlTool } from '@ai-code/curl-tool'

const myCurlTool = createCurlTool({
  assertSafeUrl: async (url) => {
    if (url.hostname === 'localhost') throw new Error('Blocked')
  }
})

const getResult = await myCurlTool.invoke({ url: 'https://example.com' })
```

Application callers should keep their injected URL policy. The relay-agent has an additional server-side execution/security boundary; do not replace it with client-side validation alone.

## Native CLI

Build the native tools:

```bash
pnpm build:tools
```

Run the current Rust CLI during development:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin curl-tool -- \
  https://example.com
```

POST example:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin curl-tool -- \
  https://httpbin.org/post \
  --request POST \
  --header 'Content-Type: application/json' \
  --data '{"hello":"world"}'
```

The Rust CLI accepts `--request`/`-X`, repeated `--header`/`-H`, `--data`/`-d`, `--timeout`, and an explicit `--no-guard` local bypass.

By default the Rust CLI enforces HTTP/HTTPS-only SSRF protections including private/local/link-local address rejection and guarded DNS resolution. `--no-guard` is for explicit local use; it must not be used to weaken relay-agent policy.

Use the binary help as the authoritative CLI reference:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin curl-tool -- --help
```

Do **not** document or rely on `npx @ai-code/curl-tool ...`; the package no longer exposes an npm CLI bin mapping.
