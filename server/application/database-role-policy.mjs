export const PRIVILEGED_DATABASE_ROLE_CAPABILITIES = Object.freeze([
  ['rolsuper', 'superuser'],
  ['rolcreaterole', 'create_role'],
  ['rolcreatedb', 'create_db'],
  ['rolreplication', 'replication'],
  ['rolbypassrls', 'bypass_rls']
])

export function unsafeRuntimeDatabaseCapabilities(role) {
  return PRIVILEGED_DATABASE_ROLE_CAPABILITIES
    .filter(([column]) => role[column] === true)
    .map(([, label]) => label)
}

export function assertLeastPrivilegeRuntimeDatabaseRole(role) {
  const unsafe = unsafeRuntimeDatabaseCapabilities(role)
  if (unsafe.length > 0) {
    throw new Error(`Unsafe runtime database role capabilities: ${unsafe.join(', ')}`)
  }
}
