import postgres from 'postgres'
import { assertLeastPrivilegeRuntimeDatabaseRole } from '../server/application/database-role-policy.mjs'

function enabled(value) {
  return String(value ?? '').toLowerCase() === 'true'
}

async function assertRuntimeDatabaseRole() {
  if (!enabled(process.env.NUXT_DATABASE_ENFORCE_LEAST_PRIVILEGE)) return
  const url = process.env.NUXT_DATABASE_URL
  if (!url) throw new Error('NUXT_DATABASE_URL is required when database least-privilege enforcement is enabled')

  const sql = postgres(url, { prepare: false, max: 1, idle_timeout: 1, connect_timeout: 5 })
  try {
    const [role] = await sql`
      select rolsuper, rolcreaterole, rolcreatedb, rolreplication, rolbypassrls
      from pg_roles
      where rolname = current_user
    `
    if (!role) throw new Error('Unable to inspect runtime database role')
    assertLeastPrivilegeRuntimeDatabaseRole(role)
  } finally {
    await sql.end({ timeout: 1 }).catch(() => undefined)
  }
}

await assertRuntimeDatabaseRole()
await import('../.output/server/index.mjs')
