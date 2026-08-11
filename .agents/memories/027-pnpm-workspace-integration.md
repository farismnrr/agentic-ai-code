# 027 pnpm/workspace integration

## Fresh-checkout Build/Install Workflow
To ensure a seamless developer experience that matches a pure JavaScript monorepo, we've integrated the Rust build step directly into the standard `pnpm install` lifecycle.

The root `package.json` now includes a `build:tools` script that executes `cargo build --release`. This script is hooked into `postinstall`, meaning that any time a developer runs `pnpm install` on a fresh checkout (or after pulling new changes), the Rust CLIs are automatically compiled and placed into `target/release/`.

```json
  "scripts": {
    "build:tools": "cargo build --release",
    "postinstall": "pnpm build:tools && nuxt prepare"
  }
```

## Binary Resolution Strategy
With the TS CLIs migrating to Rust, the `packages/*/src/index.ts` files (which serve as the Nuxt application's LangChain tool factories) no longer execute tool logic natively. Instead, they act as process wrappers that invoke the compiled Rust binaries.

To avoid hardcoded developer absolute paths (which would break on other machines or in production environments), the TS factories resolve the Rust binary dynamically based on their own location:

```typescript
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const rustBin = path.join(__dirname, '../../../target/release/tool-name')
```

This ensures that the resolution correctly climbs out of `packages/<tool-name>/src/` to the workspace root `target/release/` directory regardless of where the repository is cloned.

## Verification of No JS Fallbacks
All fallback mechanisms have been removed to enforce the Rust-only invariant:
1. `cli.mjs` entrypoints were deleted in previous PR stages and `package.json` `bin` mappings were updated.
2. The legacy `fetch`/`execa` application logic inside the tool factories (`curl-tool`, `searxng-search-tool`, and `terminal-tool`) was stripped out and replaced with explicit `execa(rustBin, ...)` spawns targeting the Rust CLIs.
3. The `USE_RUST_CLI` environment variable branches in the test JS Oracles (`tests/*_cli.js`) were removed. The testing ecosystem now relies directly on the TS factories which themselves delegate directly to Rust.
