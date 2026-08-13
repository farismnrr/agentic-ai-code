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

// --- Part 2: deterministic redirect policy coverage -------------------------
const responses = new Map()
const seen = []
const resolveFixture = async hostname => [{ address: hostname === 'private.example' ? '127.0.0.1' : '93.184.216.34', family: 4 }]
const fixtureFetch = async (url, init) => {
  seen.push({ url: url.toString(), headers: new Headers(init?.headers) })
  return responses.get(url.toString()) ?? new Response('ok', { status: 200 })
}
const redirect = location => new Response(null, { status: 302, headers: { location } })
const safeFetch = name => createSsrfSafeFetch(name, { fetch: fixtureFetch, resolve: resolveFixture })
const headers = { Authorization: 'secret', 'x-api-key': 'secret', 'x-custom-secret': 'secret' }

responses.set('https://provider.example/start', redirect('https://private.example/landing'))
let rejected = false
try { await safeFetch('private redirect')('https://provider.example/start', { headers }) } catch { rejected = true }
if (!rejected || seen.length !== 1) fail('private redirect was not rejected before follow-up fetch')

responses.clear(); seen.length = 0
responses.set('https://provider.example/start', redirect('https://other.example/landing'))
rejected = false
try { await safeFetch('cross-origin redirect')('https://provider.example/start', { headers }) } catch { rejected = true }
if (!rejected || seen.length !== 1) fail('cross-origin redirect was accepted')

responses.clear(); seen.length = 0
responses.set('https://provider.example/start', redirect('https://provider.example/landing'))
if ((await safeFetch('same-origin redirect')('https://provider.example/start', { headers })).status !== 200 || seen.length !== 2) fail('same-origin redirect did not succeed')

responses.clear(); seen.length = 0
responses.set('https://provider.example/start', redirect('http://provider.example/landing'))
rejected = false
try { await safeFetch('downgrade')('https://provider.example/start') } catch { rejected = true }
if (!rejected) fail('HTTPS downgrade redirect was accepted')

responses.clear(); seen.length = 0
responses.set('https://provider.example/start', redirect('https://provider.example/start'))
rejected = false
try { await safeFetch('redirect loop')('https://provider.example/start') } catch { rejected = true }
if (!rejected || seen.length !== 6) fail('redirect hop limit was not enforced')

console.log('phase9-ssrf-redirect-guard: all checks passed')
NODE
