#!/usr/bin/env node

import { parseArgs } from 'node:util'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { createSearxngSearchTool } from '../src/index.ts'

if (process.env.USE_RUST_CLI === '1') {
  const __dirname = path.dirname(fileURLToPath(import.meta.url))
  const rustBin = path.join(__dirname, '../../../target/release/searxng-search-tool')

  const { status, error } = spawnSync(rustBin, process.argv.slice(2), {
    stdio: 'inherit',
    env: process.env
  })

  if (error) {
    const debugBin = path.join(__dirname, '../../../target/debug/searxng-search-tool')
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
    'base-url': { type: 'string', default: 'http://127.0.0.1:8888' }
  },
  allowPositionals: true
})

const query = positionals[0]
if (!query) {
  console.error('Usage: searxng-search-tool <query> [--base-url <url>]')
  process.exit(1)
}

const t = createSearxngSearchTool({ baseUrl: values['base-url'] })

t.invoke({ query }).then(res => console.log(res)).catch(err => console.error(err))
