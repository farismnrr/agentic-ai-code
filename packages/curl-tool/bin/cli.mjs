#!/usr/bin/env node

import { parseArgs } from 'node:util'
import { createCurlTool } from '../src/index.ts'

const { values, positionals } = parseArgs({
  args: process.argv.slice(2),
  options: {
    request: { type: 'string', short: 'X', default: 'GET' },
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
