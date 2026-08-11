#!/usr/bin/env node

import { parseArgs } from 'node:util'
import { createSearxngSearchTool } from '../../searxng-search-tool/src/index.ts'

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
