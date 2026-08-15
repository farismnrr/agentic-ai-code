# Plan 036 Phase 0 — MCP Compatibility Matrix

Date audited: 2026-08-15

This document records the Phase 0 source/specification re-audit before runtime implementation. Current source and current official specifications outrank historical Plan 028/029 wording when they disagree.

## Compatibility matrix

| Surface | Current repository | Current target | Gap / action |
| --- | --- | --- | --- |
| Laptop MCP transport | Rust relay exposes `POST /mcp`, modern `server/discover`, strict 2026 routing metadata, and narrow legacy compatibility | MCP `2026-07-28` Streamable HTTP | Keep the Rust resource server; do not invent a second terminal protocol. |
| Public resource identity | Remote mode already validates a configured HTTPS OAuth audience/resource and publishes Protected Resource Metadata | `https://mcp.farismunir.my.id/mcp` | Deployment must configure this exact public resource and ensure discovery never advertises loopback addresses. |
| Relay authorization role | Resource Server only; validates asymmetric JWT issuer/audience/time/owner/scope and advertises `relay.coding` | MCP OAuth Resource Server with separate Authorization Server | Preserve this split. Do not move authorization-code/token issuance into the relay. |
| external MCP client OAuth client | Relay already exposes resource metadata, tool OAuth schemes, and challenges expected by remote clients | external MCP client current MCP OAuth flow: OAuth 2.1, PKCE, resource binding; CIMD preferred with DCR/pre-registered compatibility where available | Remaining work is external Authorization Server + live external MCP client proof, not a relay-side login implementation. |
| Hosted Nuxt MCP client | `server/infrastructure/mcp/client.ts` uses the MCP TypeScript SDK Streamable HTTP client but had no bearer/OAuth credential path | Same public MCP resource used by external MCP client | Add a private server-side first-party credential path keyed by exact resource URL; keep secrets out of browser/API/DB surfaces. |
| Nuxt protocol era | Repository currently depends on `@modelcontextprotocol/sdk` v1 because the separate legacy inbound `/api/mcp` server also uses it | MCP TypeScript SDK v2 is the current 2026-capable client line | Do not mix the package migration into the first auth-boundary change without a lockfile + local gate. The Rust relay retains narrow legacy client compatibility meanwhile. |
| Redirect / SSRF boundary | Outbound MCP validated only the initial URL before handing networking to the SDK | Every credential-bearing hop must remain on an allowed public origin | Route Streamable HTTP through the existing `createSsrfSafeFetch()` redirect/DNS guard; reject cross-origin/downgrade redirects. |
| Trace continuity | Rust relay can extract W3C trace context; generic Nuxt MCP client did not explicitly inject it | Nuxt -> public MCP -> Rust trace continuity for the first-party relay | Inject active trace context only for the configured first-party resource, not arbitrary third-party MCP servers. |
| Settings verification | Server has `/api/mcp-servers/:id/test`, but the MCP settings UI had no control wired to it | Operator can verify the public endpoint and populate tool discovery before using it in chat | Wire a Test action that updates status/tools and reports bounded failures. |
| Public deployment acceptance | Old external MCP client acceptance script referenced pre-layered Rust paths and only made a weak metadata probe | Current layered paths + resource-path Protected Resource Metadata validation | Repair the script and ensure it never labels a metadata-only probe as a full external MCP client E2E success. |
| Tunnel / TLS | Not provisioned by repository source | Outbound-established tunnel, stable HTTPS public hostname, relay remains loopback-only | External deployment step; no code change may broaden relay binding to `0.0.0.0`. |
| Authorization Server | Not implemented in repo by design | Established standards-compliant AS/IdP supporting current MCP + external MCP client requirements | External integration decision remains open. Prefer an established IdP over writing an OAuth server here. |

## Phase 0 decisions

1. **Canonical resource:** `https://mcp.farismunir.my.id/mcp` remains the target resource/audience unless deployment constraints force a reviewed change.
2. **One MCP resource:** Nuxt and external MCP client consume the same Rust relay MCP resource; Nuxt does not gain a private shell endpoint.
3. **First Nuxt credential slice:** hosted Nuxt may attach an externally-issued OAuth access token from private runtime configuration only when the stored MCP URL exactly matches the configured first-party resource. This is not an Authorization Server and not a substitute for external MCP client's interactive OAuth flow.
4. **Credential redirect rule:** authenticated MCP requests must use the repository's existing same-origin SSRF-safe redirect walker so a public endpoint cannot redirect the Bearer token into another origin or private network.
5. **Protocol package migration:** migration of the Nuxt client from `@modelcontextprotocol/sdk` v1 to the split v2 client package is a separate dependency slice because the legacy inbound Nuxt MCP endpoint still imports v1 server APIs. It must include a real lockfile update and local verification rather than a hand-edited dependency declaration.
6. **Live evidence:** neither a static source check nor a Protected Resource Metadata curl is sufficient to claim external MCP client interoperability. Final acceptance requires a real external MCP client developer-mode connection, OAuth completion, tool discovery, and tool call.

## External facts rechecked

At implementation start, official MCP documentation identifies `2026-07-28` as the modern stateless HTTP revision, requires OAuth Protected Resource Metadata for protected HTTP resources, treats the MCP server as the Resource Server, and prefers Client ID Metadata Documents while retaining compatibility mechanisms such as DCR where supported. Current OpenAI MCP documentation requires a stable public HTTPS MCP endpoint for production integrations and supports OAuth 2.1/PKCE with MCP resource binding for authenticated external MCP client connections.

These facts must be rechecked again if implementation resumes after the external specifications or OpenAI connection flow changes.
