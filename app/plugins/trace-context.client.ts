import { ofetch } from 'ofetch'

// Plan 035 Phase 4 / P1 — same-origin W3C trace continuity for user-initiated
// API actions. Uses the single reusable `createTracedFetch()` primitive
// (app/utils/trace-context.ts) as the underlying fetch for the global
// `$fetch` (the documented Nuxt pattern for a global fetch interceptor, via
// ofetch's `create(defaults, { fetch })` hook) so every same-origin
// `/api/**` request carries a fresh `traceparent` header, and records a
// bounded, sanitized success/error telemetry event correlated to that trace
// ID and the server-returned `x-request-id` response header.
//
// `traceparent` is attached ONLY when `isSameOriginApiRequest()` returns
// true — there is no code path here that can attach it to a third-party
// origin (user-configured provider base URLs, remote MCP endpoints, OAuth,
// etc.), per the plan's trust-boundary rule. The AI SDK chat transport
// (app/composables/chat/chat-transport.ts) uses the SAME primitive so trace
// generation and telemetry recording are never duplicated between the two
// call sites.
export default defineNuxtPlugin(() => {
  const telemetry = useTelemetry()
  const tracedFetch = createTracedFetch(telemetry, globalThis.fetch)

  // Built directly against ofetch's own `create(defaults, { fetch })`
  // overload (rather than the ambient `$fetch` global, whose type comes
  // from nitropack's re-export and only exposes the single-arg overload) so
  // the underlying-fetch hook resolves correctly; the result is still a
  // fully compatible `$Fetch` instance.
  // Documented Nuxt recipe for a global fetch interceptor:
  // https://nuxt.com/docs/guide/recipes/custom-fetch
  globalThis.$fetch = ofetch.create({}, { fetch: tracedFetch }) as typeof $fetch
})
