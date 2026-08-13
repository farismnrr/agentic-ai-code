import { lookup } from 'node:dns/promises'
import { isIPv4, isIPv6 } from 'node:net'

/**
 * True for addresses that shouldn't be reachable from a server-side
 * "connect to whatever URL a user typed in" feature: loopback, private
 * (RFC1918), link-local (includes the 169.254.169.254 cloud-metadata
 * address), and IPv6 equivalents.
 */
function isDisallowedIPv4(address: string) {
  const parts = address.split('.').map(Number)
  const a = parts[0]
  const b = parts[1] ?? -1
  return a === 127 || a === 10 || a === 0
    || (a === 169 && b === 254)
    || (a === 172 && b >= 16 && b <= 31)
    || (a === 192 && b === 168)
}

// Exported for direct, network-free unit coverage of the address classifier
// (see `scripts/phase9-ssrf-redirect-guard.sh`) — not otherwise used outside
// this module.
export function isDisallowedAddress(address: string) {
  if (isIPv4(address)) {
    return isDisallowedIPv4(address)
  }
  if (isIPv6(address)) {
    const normalized = address.toLowerCase()
    // IPv4-mapped (`::ffff:a.b.c.d`) and IPv4-compatible (`::a.b.c.d`)
    // IPv6 addresses embed a real IPv4 address — unwrap and re-check with
    // the same IPv4 rules instead of only pattern-matching a couple of
    // known-bad prefixes, so every RFC1918/loopback/link-local/metadata
    // IPv4 range is caught in its mapped form too.
    const mapped = normalized.match(/^::(?:ffff:)?(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})$/)
    if (mapped && mapped[1]) {
      return isDisallowedIPv4(mapped[1])
    }
    return normalized === '::1' || normalized === '::'
      || normalized.startsWith('fe80:') // link-local, includes IPv6 metadata equivalents
      || normalized.startsWith('fc') || normalized.startsWith('fd') // unique local, fc00::/7
  }
  return true // unrecognized address shape — fail closed
}

/**
 * Rejects anything that isn't a plain http(s) URL resolving to a public
 * address. Re-resolves DNS right before connecting (not just at
 * registration time) so a hostname that resolves safely at registration
 * and to an internal address later (DNS rebinding) is still caught.
 */
export async function assertSafeUrl(url: URL, context: string) {
  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    throw new Error(`${context} has an unsupported URL scheme "${url.protocol}"`)
  }

  let addresses: string[]
  try {
    addresses = (await lookup(url.hostname, { all: true })).map(a => a.address)
  } catch {
    throw new Error(`${context} has a URL that could not be resolved`)
  }

  if (addresses.length === 0 || addresses.some(isDisallowedAddress)) {
    throw new Error(`${context} resolves to a disallowed address`)
  }
}

const MAX_REDIRECT_HOPS = 5

// Headers that carry credentials for the *original* request and must not be
// silently forwarded to a different origin or downgraded scheme on redirect.
const SENSITIVE_REQUEST_HEADERS = ['authorization', 'cookie', 'proxy-authorization']

function isRedirectResponse(response: Response) {
  return [301, 302, 303, 307, 308].includes(response.status) && response.headers.has('location')
}

/**
 * A `fetch`-compatible function that validates every hop of the request —
 * including redirect targets — against `assertSafeUrl` before connecting to
 * it. Intended for handing to SDK `fetch`/`clientOptions.fetch` hooks (AI SDK
 * provider factories, the OpenAI and Anthropic Node SDKs via LangChain) so
 * the same SSRF policy already used for outbound MCP connections also covers
 * provider discovery and chat requests, not just our own hand-written
 * `fetch()` calls.
 *
 * Native `fetch`'s automatic redirect-following bypasses URL validation
 * entirely — a server can pass the initial-URL check and then 302 the
 * request into loopback/private/link-local/cloud-metadata address space.
 * This disables automatic redirects (`redirect: 'manual'`) and walks the
 * chain itself, bounded to `MAX_REDIRECT_HOPS` hops, re-validating (and
 * re-resolving DNS for) each target before following it. Credential-bearing
 * headers are stripped whenever a redirect crosses origin or downgrades
 * https -> http, so they're never silently leaked off-origin.
 *
 * Residual risk: the resolved address isn't pinned through to the actual TCP
 * connection, so DNS rebinding between this check and the real connect for a
 * given hop is not fully closed. `curl-tool`'s guard accepts the same
 * residual risk; see `.agents/memories/README.md`.
 */
export function createSsrfSafeFetch(context: string): typeof fetch {
  return async (input, init) => {
    let url = new URL(input instanceof Request ? input.url : input.toString())
    const method = init?.method ?? (input instanceof Request ? input.method : 'GET')
    const headers = new Headers(input instanceof Request ? input.headers : init?.headers)
    const body = init?.body ?? (input instanceof Request ? input.body : undefined)
    const originalOrigin = url.origin
    const originalProtocol = url.protocol

    for (let hop = 0; hop <= MAX_REDIRECT_HOPS; hop++) {
      await assertSafeUrl(url, context)
      const response = await fetch(url, { ...init, method, headers, body, redirect: 'manual' })
      if (!isRedirectResponse(response)) return response
      if (hop === MAX_REDIRECT_HOPS) {
        throw new Error(`${context} exceeded the maximum number of redirects (${MAX_REDIRECT_HOPS})`)
      }

      const nextUrl = new URL(response.headers.get('location')!, url)
      const isCrossOrigin = nextUrl.origin !== originalOrigin
      const isSchemeDowngrade = originalProtocol === 'https:' && nextUrl.protocol === 'http:'
      if (isCrossOrigin || isSchemeDowngrade) {
        for (const name of SENSITIVE_REQUEST_HEADERS) headers.delete(name)
      }
      url = nextUrl
    }
    throw new Error(`${context} exceeded the maximum number of redirects (${MAX_REDIRECT_HOPS})`)
  }
}
