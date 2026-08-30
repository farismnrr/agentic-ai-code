import { defineConfig } from 'drizzle-kit'

/**
 * Drizzle Kit configuration for schema generation and migration.
 *
 * Migrations use a dedicated `NUXT_DATABASE_MIGRATION_URL`. Development may
 * fall back to `NUXT_DATABASE_URL`, but production deliberately cannot reuse
 * the application runtime credential for DDL. The `?search_path=ai_code` suffix
 * is added by the app code; drizzle-kit connects without it and uses the
 * `schemaFilter` below instead.
 *
 * Usage:
 *   pnpm db:generate   # generate SQL migration files from schema changes
 *   pnpm db:migrate    # apply pending migrations to the database
 */
const migrationUrl = process.env.NUXT_DATABASE_MIGRATION_URL
  ?? (process.env.NODE_ENV === 'production' ? '' : process.env.NUXT_DATABASE_URL)
  ?? ''

export default defineConfig({
  schema: './server/database/schema.ts',
  out: './server/database/migrations',
  dialect: 'postgresql',
  dbCredentials: {
    url: migrationUrl
  },
  schemaFilter: ['ai_code']
})
