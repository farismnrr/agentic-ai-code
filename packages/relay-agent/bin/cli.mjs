#!/usr/bin/env node

import { parseArgs } from 'node:util'
import { RelayAgentServer } from '../src/index.ts'

const { values } = parseArgs({
  args: process.argv.slice(2),
  options: {
    port: { type: 'string', short: 'p', default: '47821' },
    dir: { type: 'string', short: 'd', default: process.cwd() }
  },
  allowPositionals: true
})

const port = parseInt(values.port ?? '47821', 10)
if (isNaN(port)) {
  console.error('Invalid port specified')
  process.exit(1)
}

const server = new RelayAgentServer({
  port,
  dir: values.dir
})

server.start().catch((err) => {
  console.error('[relay-agent] Failed to start server:', err)
  process.exit(1)
})
