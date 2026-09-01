---
name: searxng-search-tool
description: LangChain SearXNG search tool plus its native Rust CLI.
license: MIT
---

# @ai-code/searxng-search-tool

`@ai-code/searxng-search-tool` provides the TypeScript LangChain search tool used by the application. The standalone executable is now the native Rust unified `ai-tools` binary in [`../rust-tools/`](../rust-tools/), not an npm `bin`.

## TypeScript usage

```ts
import { createSearxngSearchTool } from '@ai-code/searxng-search-tool'

const searchTool = createSearxngSearchTool({
  baseUrl: 'http://127.0.0.1:8888'
})

const results = await searchTool.invoke({ query: 'latest tech news' })
console.log(results)
```

## Native CLI

Build the native tools:

```bash
pnpm build:tools
```

Run a search with the current Rust CLI:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin ai-tools -- searxng \
  'how to build a web scraper'
```

Specify another SearXNG endpoint with `--base-url`:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin ai-tools -- searxng \
  'Nuxt documentation' \
  --base-url https://searx.example.com
```

The CLI default remains `http://127.0.0.1:8888`. It requests `/search?q=...&format=json`, applies a bounded request timeout, and returns up to the first 10 formatted results.

Use the binary help as the authoritative CLI reference:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin ai-tools -- searxng --help
```

Do **not** document or rely on `npx @ai-code/searxng-search-tool ...`; the package no longer exposes an npm CLI bin mapping.
