#!/usr/bin/env node

import { parseArgs } from 'node:util'
import { RelayAgentServer } from '../src/index.ts'
import { acquireLock, isProcessAlive, readPidFile, removePidFile, removePidFileIfOwnedByMe } from './pidfile.mjs'

const { values, positionals } = parseArgs({
  args: process.argv.slice(2),
  options: {
    port: { type: 'string', short: 'p', default: '47821' },
    // No hardcoded default here either — RelayAgentServer defaults to
    // os.homedir() (see src/server.ts). `--dir` is only a starting-point
    // cwd, not a restriction: this agent has no directory jail by design
    // (full access to whatever this OS user can reach; the paired browser
    // session and per-command chat approval are the actual gates).
    dir: { type: 'string', short: 'd' },
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

const port = parseInt(values.port ?? '47821', 10)
if (isNaN(port)) {
  console.error('Invalid port specified')
  process.exit(1)
}

if (positionals[0] === 'stop') {
  const pid = readPidFile(port)
  if (!pid || !isProcessAlive(pid)) {
    // A pidfile from a process that crashed/was SIGKILLed without cleanup
    // shouldn't leave `stop` unable to tell the difference from "nothing to
    // do here" — treat a stale file the same as no file at all.
    if (pid) removePidFile(port)
    console.log(`[relay-agent] No running agent found on port ${port}.`)
    process.exit(0)
  }
  process.kill(pid, 'SIGTERM')
  console.log(`[relay-agent] Stop signal sent to pid ${pid} (port ${port}).`)
  // The target process removes its own pidfile as part of its SIGTERM
  // handler below — nothing left to clean up here.
  process.exit(0)
}

const origin = values.origin ?? process.env.RELAY_AGENT_ORIGIN

// The actual concurrency guard — acquired *before* ever touching the
// network, not inferred afterward from who won the `listen()` race (see
// `acquireLock`'s own comment in pidfile.mjs for why that was fragile in
// practice: writing the pidfile only after a successful bind still left a
// window where a second process could observe a stale-but-not-yet-cleared
// lock and misjudge it). A clean, immediate refusal here is also just
// better UX than surfacing a raw `EADDRINUSE` stack trace for what is, from
// the user's perspective, "you already have one of these running."
if (!acquireLock(port)) {
  const existingPid = readPidFile(port)
  console.error(`[relay-agent] Already running on port ${port} (pid ${existingPid}). Use \`relay-agent stop\` first, or pass a different --port.`)
  process.exit(1)
}

const server = new RelayAgentServer({
  port,
  dir: values.dir,
  allowedOrigin: origin
})

// Without this, Ctrl+C (SIGINT) — and `relay-agent stop`'s SIGTERM above —
// just killed the process outright: no pidfile cleanup (leaving `stop`
// unable to tell a dead agent from a live one next time), no closing the
// WebSocket/HTTP server, no confirmation the shutdown actually happened.
let shuttingDown = false
async function shutdown(signal) {
  if (shuttingDown) return
  shuttingDown = true
  console.log(`\n[relay-agent] Received ${signal}, shutting down...`)
  removePidFileIfOwnedByMe(port)
  await server.stop()
  console.log('[relay-agent] Stopped.')
  process.exit(0)
}
process.on('SIGINT', () => void shutdown('SIGINT'))
process.on('SIGTERM', () => void shutdown('SIGTERM'))

server.start().catch((err) => {
  // We hold the lock but never actually bound the port (e.g. it's held by
  // some unrelated process, or a permissions error) — release it, since
  // this process is exiting and has nothing left to be a lock *for*.
  removePidFileIfOwnedByMe(port)
  console.error('[relay-agent] Failed to start server:', err)
  process.exit(1)
})
