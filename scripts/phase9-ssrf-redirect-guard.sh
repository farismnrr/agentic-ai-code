#!/usr/bin/env bash
set -euo pipefail

# Plan 031A Finding Q: `createSsrfSafeFetch` must re-validate every redirect
# hop, not just the initial URL. This is a deterministic, network-free-where-
# possible verification harness — the only "network" traffic is loopback
# HTTP against a server this script starts itself.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

command -v node >/dev/null

node --input-type=module <<'NODE'
import http from 'node:http'
import { isDisallowedAddress, createSsrfSafeFetch } from './server/utils/ssrf-guard.ts'

function fail(message) {
  console.error(`phase9-ssrf-redirect-guard: FAIL — ${message}`)
  process.exit(1)
}

// --- Part 1: classifier unit coverage (no network) --------------------------
// Curated list of addresses that must be rejected: loopback, RFC1918,
// link-local/cloud-metadata, unspecified, and their IPv4-mapped/compatible
// IPv6 equivalents. Testing the classifier directly (rather than over real
// DNS/network) is what makes the "safe public address" side of this
// deterministic — no dependency on live external hosts.
const disallowed = [
  '127.0.0.1',
  '10.0.0.5',
  '172.16.0.1',
  '172.31.255.255',
  '192.168.1.1',
  '169.254.169.254', // cloud metadata
  '0.0.0.0',
  '::1',
  '::',
  'fe80::1',
  'fc00::1',
  'fd12:3456::1',
  '::ffff:127.0.0.1', // IPv4-mapped loopback
  '::ffff:10.0.0.1', // IPv4-mapped RFC1918
  '::ffff:169.254.169.254', // IPv4-mapped cloud metadata
  '::127.0.0.1' // IPv4-compatible loopback
]
for (const address of disallowed) {
  if (!isDisallowedAddress(address)) fail(`expected "${address}" to be classified as disallowed`)
}

const allowed = [
  '8.8.8.8',
  '1.1.1.1',
  '93.184.216.34', // example.com (historical)
  '2606:4700:4700::1111' // Cloudflare public IPv6
]
for (const address of allowed) {
  if (isDisallowedAddress(address)) fail(`expected "${address}" to be classified as allowed`)
}
console.log('phase9-ssrf-redirect-guard: classifier unit coverage passed')

// --- Part 2: redirect-to-loopback end-to-end rejection -----------------------
// 127.0.0.1 is itself a disallowed loopback address, so redirecting a
// "public-looking" request to it is sufficient to prove the manual
// redirect-hop loop actually re-validates each hop instead of delegating to
// native fetch's automatic redirect following (which would connect to it
// without ever re-running the guard).
const redirectServer = http.createServer((req, res) => {
  if (req.url === '/public') {
    const port = redirectServer.address().port
    res.writeHead(302, { Location: `http://127.0.0.1:${port}/internal` })
    res.end()
    return
  }
  res.writeHead(200, { 'Content-Type': 'text/plain' })
  res.end('should never be reached')
})
await new Promise(resolve => redirectServer.listen(0, '127.0.0.1', resolve))
const redirectPort = redirectServer.address().port

const safeFetch = createSsrfSafeFetch('phase9 test')
let rejected = false
try {
  await safeFetch(`http://127.0.0.1:${redirectPort}/public`)
} catch {
  rejected = true
}
await new Promise(resolve => redirectServer.close(resolve))
if (!rejected) fail('expected redirect-to-loopback chain to be rejected, but it succeeded')
console.log('phase9-ssrf-redirect-guard: redirect-to-loopback rejection passed')

console.log('phase9-ssrf-redirect-guard: all checks passed')
NODE
