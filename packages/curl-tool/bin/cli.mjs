#!/usr/bin/env node

import { parseArgs } from 'node:util'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { createCurlTool } from '../src/index.ts'

if (process.env.USE_RUST_CLI === '1') {
  const __dirname = path.dirname(fileURLToPath(import.meta.url))
  const rustBin = path.join(__dirname, '../../../target/release/curl-tool')

  const { status, error } = spawnSync(rustBin, process.argv.slice(2), {
    stdio: 'inherit',
    env: process.env
  })

  if (error) {
    const debugBin = path.join(__dirname, '../../../target/debug/curl-tool')
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
    'request': { type: 'string', short: 'X', default: 'GET' },
    'header': { type: 'string', short: 'H', multiple: true },
    'data': { type: 'string', short: 'd' },
    'no-guard': { type: 'boolean' }
  },
  allowPositionals: true
})

const url = positionals[0]
if (!url) {
  console.error('Usage: curl-tool <url> [--request <method>] [--header <header>...] [--data <body>] [--no-guard]')
  process.exit(1)
}

const headers = {}
if (values.header) {
  for (const h of values.header) {
    const [key, ...rest] = h.split(':')
    headers[key.trim()] = rest.join(':').trim()
  }
}

const assertSafeUrl = async () => {
  if (!values['no-guard']) {
    console.warn('WARN: SSRF guard is enabled but no external validation is provided in CLI. Pass --no-guard if you want to bypass SSRF protection.')
    throw new Error('SSRF guard blocked request. Use --no-guard to bypass.')
  }
}

const t = createCurlTool({ assertSafeUrl })

t.invoke({
  url,
  method: values.request,
  headers: Object.keys(headers).length > 0 ? headers : undefined,
  body: values.data
}).then(res => console.log(res)).catch(err => console.error(err))
