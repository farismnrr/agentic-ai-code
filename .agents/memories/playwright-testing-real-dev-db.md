---
name: playwright-testing-real-dev-db
description: automated Playwright verification against the local dev server hits the real shared dev database — 5 real conversations were lost during one debugging session, cause never pinned down.
metadata:
  type: feedback
---

Live-verifying fixes with Playwright (logging in, sending messages, reloading) during this session's `@search`/sidebar debugging ran against the actual `sensio-postgres` dev database, not an isolated test DB. At some point during repeated dev-server restarts and automated interactions, 5 real conversations in the "bnsp" workspace were deleted from the DB — the exact trigger was never identified (dev.log had been truncated multiple times across restarts, losing the evidence).

**Why this matters:** nothing in this repo's dev setup isolates Playwright-driven verification from real data. `pnpm dev`'s DB connection is the same one a human would use.

**How to apply:** before running Playwright scripts that log in and interact with a real account against this dev server, prefer a disposable/seeded test workspace over an account with real conversations, or snapshot the DB first if that's not practical. If data loss is ever noticed again, check `pg_stat_archiver` on `sensio-postgres` immediately (WAL archiving is on — PITR may be possible) before concluding it's unrecoverable.
