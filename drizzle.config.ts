import { defineConfig } from 'drizzle-kit'

/**
 * Drizzle Kit configuration for schema generation and migration.
 *
 * The connection URL is read from `NUXT_DATABASE_URL` so the same env file
 * that runs the app also drives migrations. The `?search_path=ai_code` suffix
 * is added by the app code; drizzle-kit connects without it and uses the
 * `schemaFilter` below instead.
 *
 * Usage:
 *   pnpm db:generate   # generate SQL migration files from schema changes
 *   pnpm db:migrate    # apply pending migrations to the database
 */
export default defineConfig({
  schema: './server/database/schema.ts',
  out: './server/database/migrations',
  dialect: 'postgresql',
  dbCredentials: {
    url: process.env.NUXT_DATABASE_URL ?? ''
  },
  schemaFilter: ['ai_code']
})
