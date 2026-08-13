import { lookup } from 'node:dns/promises'
import { isIPv4, isIPv6 } from 'node:net'

/**
 * True for addresses that shouldn't be reachable from a server-side
 * "connect to whatever URL a user typed in" feature: loopback, private
 * (RFC1918), link-local (includes the 169.254.169.254 cloud-metadata
 * address), and IPv6 equivalents.
 */
function isDisallowedAddress(address: string) {
  if (isIPv4(address)) {
    const [a, b] = address.split('.').map(Number)
    return a === 127 || a === 10 || a === 0
      || (a === 169 && b === 254)
      || (a === 172 && b !== undefined && b >= 16 && b <= 31)
      || (a === 192 && b === 168)
  }
  if (isIPv6(address)) {
    const normalized = address.toLowerCase()
    return normalized === '::1' || normalized === '::'
      || normalized.startsWith('fe80:') // link-local, includes IPv6 metadata equivalents
      || normalized.startsWith('fc') || normalized.startsWith('fd') // unique local, fc00::/7
      || normalized.startsWith('::ffff:127.') || normalized.startsWith('::ffff:169.254.') // IPv4-mapped loopback/link-local
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

/**
 * A `fetch`-compatible function that re-validates the request URL against
 * `assertSafeUrl` immediately before every call. Intended for handing to SDK
 * `fetch`/`clientOptions.fetch` hooks (AI SDK provider factories, the OpenAI
 * and Anthropic Node SDKs via LangChain) so the same SSRF policy already
 * used for outbound MCP connections also covers provider discovery and chat
 * requests, not just our own hand-written `fetch()` calls.
 *
 * Residual risk: this validates the request URL, not every redirect hop the
 * underlying SDK/runtime `fetch` may follow internally, and the resolved
 * address isn't pinned through to the actual TCP connection — so DNS
 * rebinding between this check and the real connect is not fully closed.
 * `curl-tool`'s guard accepts the same residual risk for its initial
 * request; see `.agents/memories/README.md`.
 */
export function createSsrfSafeFetch(context: string): typeof fetch {
  return async (input, init) => {
    const url = new URL(input instanceof Request ? input.url : input.toString())
    await assertSafeUrl(url, context)
    return fetch(input, init)
  }
}
