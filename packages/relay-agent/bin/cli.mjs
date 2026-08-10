#!/usr/bin/env node

import { parseArgs } from 'node:util'
import { RelayAgentServer } from '../src/index.ts'

const { values } = parseArgs({
  args: process.argv.slice(2),
  options: {
    port: { type: 'string', short: 'p', default: '47821' },
    dir: { type: 'string', short: 'd', default: process.cwd() },
    // No hardcoded default here — RelayAgentServer's constructor is the
    // one place that owns the fallback (see src/server.ts), so the literal
    // origin string exists in exactly one spot in this package. This CLI
    // runs on the user's own machine as its own process and has no way to
    // read the web app's actual runtime config
    // (`runtimeConfig.public.siteUrl` / `NUXT_PUBLIC_SITE_URL`) — a hosted
    // deployment must pass the real origin explicitly, either via
    // `--origin https://your-app.example.com` or once via the
    // `RELAY_AGENT_ORIGIN` env var (the web UI's Settings → Local Terminal
    // page shows the exact value to use).
    origin: { type: 'string', short: 'o' }
  },
  allowPositionals: true
})

const origin = values.origin ?? process.env.RELAY_AGENT_ORIGIN

const port = parseInt(values.port ?? '47821', 10)
if (isNaN(port)) {
  console.error('Invalid port specified')
  process.exit(1)
}

const server = new RelayAgentServer({
  port,
  dir: values.dir,
  allowedOrigin: origin
})

server.start().catch((err) => {
  console.error('[relay-agent] Failed to start server:', err)
  process.exit(1)
})
