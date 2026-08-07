# pnpm was chosen over bun and npm

Deliberate choice at project setup, not a default. pnpm is what the Nuxt and Nuxt UI docs assume, so commands copied from docs work as-is, and `nuxi` generates hoisting config that suits it.

bun was the main alternative and is faster, but has rough edges with Nuxt postinstall steps and native deps (sharp for `@nuxt/image`, better-sqlite3). That trade wasn't worth it here.

The choice is enforced by `packageManager` in `package.json`. Using another manager desyncs `pnpm-lock.yaml`.
