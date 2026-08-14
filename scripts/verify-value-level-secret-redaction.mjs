#!/usr/bin/env node
// Plan 035 remediation round 2 — P1 value-level secret redaction canary.
//
// Unlike verify-no-secret-leakage.sh (which checks that FORBIDDEN attribute
// KEYS are dropped by the allowlist), this script proves that a canary
// secret embedded INSIDE the VALUE of an ALLOWED key (e.g. `error.message`,
// `stack`) never survives sanitization unredacted. Deterministic, no
// running server required — calls the sanitizer directly.
//
// Usage: node scripts/verify-value-level-secret-redaction.mjs
// (requires tsx, already a devDependency, invoked via npx)

import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, '..')

const CANARY = 'canary-secret-fake-token-DO-NOT-LEAK-12345'

const script = `
import { sanitizeAttributes } from '${join(root, 'server/infrastructure/observability/sanitize.ts')}'
const CANARY = '${CANARY}'
const cases = [
  ['postgres DB URL in error.message', { 'error.message': 'connection failed: postgres://user:' + CANARY + '@localhost/db' }],
  ['Bearer token in error.message', { 'error.message': 'Authorization: Bearer ' + CANARY }],
  ['x-api-key assignment in error.message', { 'error.message': 'request failed x-api-key=' + CANARY }],
  ['canary embedded in stack', { stack: 'Error: boom\\n    at auth (token=' + CANARY + ')' }],
]
let allPass = true
for (const [label, attrs] of cases) {
  const out = sanitizeAttributes(attrs)
  const raw = JSON.stringify(out)
  const leaked = raw.includes(CANARY)
  if (leaked) allPass = false
  console.log(JSON.stringify({ label, out, verdict: leaked ? 'FAIL' : 'PASS' }))
}
process.exit(allPass ? 0 : 1)
`

const tmpFile = join(root, 'node_modules', '.tmp-value-redaction-canary.mjs')
const fs = await import('node:fs')
fs.writeFileSync(tmpFile, script)

let tsResult = 'PASS'
try {
  const out = execFileSync('npx', ['tsx', tmpFile], { cwd: root, encoding: 'utf8' })
  console.log(out)
} catch (err) {
  tsResult = 'FAIL'
  console.error(err.stdout ?? err.message)
} finally {
  fs.rmSync(tmpFile, { force: true })
}

// Black-box, not a unit test: builds and runs a tiny example binary that
// calls the real compiled redact_secrets()/safe_log_field() functions and
// prints their output, so this stays a deterministic acceptance script per
// repository policy (no #[cfg(test)] / unit-test suite).
let rustResult = 'PASS'
const rustCases = [
  ['safe_log_field', `Authorization: Bearer ${CANARY}`, 'Bearer [REDACTED]'],
  ['redact_secrets', `postgres://user:${CANARY}@localhost/db`, 'postgres://[REDACTED]@localhost/db'],
  ['redact_secrets', `x-api-key=${CANARY}`, '[REDACTED]']
]
for (const [mode, input, expectSubstring] of rustCases) {
  try {
    const out = execFileSync(
      'cargo',
      ['run', '--quiet', '--example', 'redact_check', '--', mode, input],
      { cwd: join(root, 'packages/rust-tools/infrastructure'), encoding: 'utf8' }
    ).trim()
    const leaked = out.includes(CANARY)
    const masked = out.includes(expectSubstring)
    const verdict = !leaked && masked ? 'PASS' : 'FAIL'
    if (verdict === 'FAIL') rustResult = 'FAIL'
    console.log(JSON.stringify({ mode, input, out, verdict }))
  } catch (err) {
    rustResult = 'FAIL'
    console.error(err.stdout ?? err.message)
  }
}

console.log(`\nTypeScript redaction canary: ${tsResult}`)
console.log(`Rust redaction canary: ${rustResult}`)

if (tsResult !== 'PASS' || rustResult !== 'PASS') process.exit(1)
