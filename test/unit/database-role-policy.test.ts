import assert from 'node:assert/strict'
import test from 'node:test'
import {
  assertLeastPrivilegeRuntimeDatabaseRole,
  unsafeRuntimeDatabaseCapabilities
} from '../../server/application/database-role-policy.mjs'

const safe = { rolsuper: false, rolcreaterole: false, rolcreatedb: false, rolreplication: false, rolbypassrls: false }

test('least-privilege runtime role is accepted', () => {
  assert.deepEqual(unsafeRuntimeDatabaseCapabilities(safe), [])
  assert.doesNotThrow(() => assertLeastPrivilegeRuntimeDatabaseRole(safe))
})

test('privileged database capabilities fail closed', () => {
  for (const capability of Object.keys(safe) as Array<keyof typeof safe>) {
    const role = { ...safe, [capability]: true }
    assert.throws(() => assertLeastPrivilegeRuntimeDatabaseRole(role), /Unsafe runtime database role capabilities/)
  }
})
