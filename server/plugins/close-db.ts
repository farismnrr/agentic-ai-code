import { closeDb } from "../infrastructure/database/connection"
/**
 * Closes the Postgres pool on graceful shutdown (SIGTERM/SIGINT), which
 * Nitro's `close` hook fires for. Without this, a redeploy or container
 * restart kills the process mid-connection instead of draining it cleanly.
 */
export default defineNitroPlugin((nitroApp) => {
  nitroApp.hooks.hook('close', () => closeDb())
})
