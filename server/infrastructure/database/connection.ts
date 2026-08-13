import postgres from 'postgres'
import { drizzle } from 'drizzle-orm/postgres-js'
import * as schema from '../../database/schema'

/**
 * Singleton postgres.js connection.
 *
 * - Uses `useRuntimeConfig()` so the URL comes from the Nuxt runtime config
 *   chain (nuxt.config.ts → NUXT_DATABASE_URL env var) rather than a bare
 *   `process.env` read, which is the idiomatic pattern in Nitro server routes.
 * - `search_path=ai_code` so every unqualified table reference in queries
 *   resolves to the ai_code schema without needing schema-qualified names.
 * - `prepare: false` is required for postgres.js with Nitro edge runtimes
 *   (no-op for Node, harmless).
 * - `idle_timeout: 20` lets an idle connection close itself after 20s.
 *   Without it the socket stays open forever, which keeps Node's event loop
 *   alive — harmless for a long-running server, but it means `nuxt build`
 *   hangs indefinitely after prerendering finishes if prerendering ever
 *   touches `useDb()` (the process never exits on its own, only on a forced
 *   kill). `closeDb()` below covers the graceful-shutdown case explicitly;
 *   this covers every other case, including build/prerender.
 *
 * The instance is module-scoped so it's shared across requests within one
 * Nitro worker. Nitro does not share module scope across workers, so this
 * does not create a cross-request leak.
 */

let _client: ReturnType<typeof postgres> | null = null
let _db: ReturnType<typeof drizzle<typeof schema>> | null = null

export function useDb() {
  if (!_db) {
    const config = useRuntimeConfig()
    const url = config.databaseUrl
    if (!url) {
      throw new Error(
        'NUXT_DATABASE_URL is not set. Add it to your .env file:\n'
        + '  NUXT_DATABASE_URL=postgres://postgres:devpassword@localhost:5432/masihawam?search_path=ai_code'
      )
    }
    _client = postgres(url, { prepare: false, idle_timeout: 20 })
    _db = drizzle(_client, { schema })
  }
  return _db
}

/** Closes the pool on graceful shutdown — see `server/plugins/close-db.ts`. */
export async function closeDb() {
  if (_client) {
    await _client.end()
    _client = null
    _db = null
  }
}
