import { strict as assert } from 'node:assert'
import { sanitizeAttributes } from '../../server/infrastructure/observability/sanitize.ts'

const canary = 'canary-secret-fake-token-DO-NOT-LEAK-12345'
const cases = [
  ['postgres DB URL in error.message', { 'error.message': `connection failed: postgres://user:${canary}@localhost/db` }],
  ['Bearer token in error.message', { 'error.message': `Authorization: Bearer ${canary}` }],
  ['x-api-key assignment in error.message', { 'error.message': `request failed x-api-key=${canary}` }],
  ['canary embedded in stack', { stack: `Error: boom\n    at auth (token=${canary})` }]
]

for (const [label, attrs] of cases) {
  const serialized = JSON.stringify(sanitizeAttributes(attrs))
  assert.equal(serialized.includes(canary), false, `${label} leaked the canary`)
}
