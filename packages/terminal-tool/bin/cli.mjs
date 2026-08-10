#!/usr/bin/env node

import { parseArgs } from 'node:util'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { createTerminalTool } from '../src/index.ts'

if (process.env.USE_RUST_CLI === '1') {
  const __dirname = path.dirname(fileURLToPath(import.meta.url))
  // Default to release, fallback to debug for dev convenience
  const rustBin = path.join(__dirname, '../../../target/release/terminal-tool')

  const { status, error } = spawnSync(rustBin, process.argv.slice(2), {
    stdio: 'inherit',
    env: process.env
  })

  if (error) {
    // Fallback to debug build if release doesn't exist
    const debugBin = path.join(__dirname, '../../../target/debug/terminal-tool')
    const debugRes = spawnSync(debugBin, process.argv.slice(2), { stdio: 'inherit', env: process.env })
    if (debugRes.error) {
      console.error(`Failed to execute Rust CLI: ${debugRes.error.message}`)
      process.exit(1)
    }
    process.exit(debugRes.status ?? 0)
  }
  process.exit(status ?? 0)
}

const { values, positionals } = parseArgs({
  args: process.argv.slice(2),
  options: {
    'cwd': { type: 'string', default: process.cwd() },
    'no-guard': { type: 'boolean' }
  },
  allowPositionals: true
})

const command = positionals[0]
const args = positionals.slice(1)

if (!command) {
  console.error('Usage: terminal-tool <command> [args...] [--cwd <path>] [--no-guard]')
  process.exit(1)
}

const assertSafeCommand = async () => {
  if (!values['no-guard']) {
    console.warn('WARN: Exec guard is enabled but no external validation is provided in CLI. Pass --no-guard if you want to bypass exec protection.')
    throw new Error('Exec guard blocked request. Use --no-guard to bypass.')
  }
}

const t = createTerminalTool({ assertSafeCommand, cwd: values.cwd })

t.invoke({
  command,
  args
}).then(res => console.log(res)).catch(err => console.error(err))
