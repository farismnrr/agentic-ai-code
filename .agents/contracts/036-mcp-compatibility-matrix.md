# Plan 036 Phase 0 — MCP Compatibility Matrix

Date audited: 2026-08-15

This document records the Phase 0 source/specification re-audit before runtime implementation. Current source and current official specifications outrank historical Plan 028/029 wording when they disagree.

## Compatibility matrix

| Surface | Current repository | Current target | Gap / action |
| --- | --- | --- | --- |
| Laptop MCP transport | Rust relay exposes `POST /mcp`, modern `server/discover`, strict 2026 routing metadata, and narrow legacy handshake/list compatibility | MCP `2026-07-28` Streamable HTTP | Keep the Rust resource server strict for normal tool RPCs; do not invent a second terminal protocol. |
| Public resource identity | Remote mode already validates a configured HTTPS OAuth audience/resource and publishes Protected Resource Metadata | `https://mcp.farismunir.my.id/mcp` | Deployment must configure this exact public resource and ensure discovery never advertises loopback addresses. |
| Relay authorization role | Resource Server only; validates asymmetric JWT issuer/audience/time/owner/scope and advertises `relay.coding` | MCP OAuth Resource Server with separate Authorization Server | Preserve this split. Do not move authorization-code/token issuance into the relay. |
| ChatGPT OAuth client | Relay already exposes resource metadata, tool OAuth schemes, and challenges expected by remote clients | ChatGPT current MCP OAuth flow: OAuth 2.1, PKCE, resource binding; CIMD preferred with DCR/pre-registered compatibility where available | Remaining work is external Authorization Server + live ChatGPT proof, not a relay-side login implementation. |
| Hosted Nuxt MCP client | Generic third-party MCP integrations still use SDK v1, while the exact first-party remote resource now has a dedicated infrastructure adapter | Same public MCP resource used by ChatGPT | Use a private server-side first-party credential path keyed by exact resource URL; keep secrets out of browser/API/DB surfaces. |
| Nuxt protocol era | Repository still depends on monolithic `@modelcontextprotocol/sdk` v1 because the separate legacy inbound `/api/mcp` server and third-party integrations use it | MCP TypeScript SDK v2 is the current 2026-capable client line | The first-party relay adapter speaks MCP `2026-07-28` directly for `server/discover`, `tools/list`, and `tools/call`, so Rust remains strict and Nuxt does not depend on v1 lifecycle semantics for the public relay. |
| Redirect / SSRF boundary | Outbound MCP validated only the initial URL before handing networking to the SDK | Every credential-bearing hop must remain on an allowed public origin | Route Streamable HTTP through the existing `createSsrfSafeFetch()` redirect/DNS guard; reject cross-origin/downgrade redirects. |
| Trace continuity | Rust relay can extract W3C trace context; generic Nuxt MCP client did not explicitly inject it | Nuxt -> public MCP -> Rust trace continuity for the first-party relay | Inject active trace context only for the configured first-party resource, not arbitrary third-party MCP servers. |
| Settings verification | Server has `/api/mcp-servers/:id/test`, but the MCP settings UI had no control wired to it | Operator can verify the public endpoint and populate tool discovery before using it in chat | Wire a Test action that updates status/tools and reports bounded failures. |
| Public deployment acceptance | Old ChatGPT acceptance script referenced pre-layered Rust paths and only made a weak metadata probe | Current layered paths + resource-path Protected Resource Metadata validation | Repair the script and ensure it never labels a metadata-only probe as a full ChatGPT E2E success. Add a no-tool public smoke probe for OAuth challenge and optional authenticated discover/tools-list. |
| Tunnel / TLS | Not provisioned by repository source | Outbound-established tunnel, stable HTTPS public hostname, relay remains loopback-only | External deployment step; no code change may broaden relay binding to `0.0.0.0`. |
| Authorization Server | Not implemented in repo by design | Established standards-compliant AS/IdP supporting current MCP + ChatGPT requirements | External integration decision remains open. Prefer an established IdP over writing an OAuth server here. |

## Phase 0 decisions

1. **Canonical resource:** `https://mcp.farismunir.my.id/mcp` remains the target resource/audience unless deployment constraints force a reviewed change.
2. **One MCP resource:** Nuxt and ChatGPT consume the same Rust relay MCP resource; Nuxt does not gain a private shell endpoint.
3. **First Nuxt credential slice:** hosted Nuxt may attach an externally-issued OAuth access token from private runtime configuration only when the stored MCP URL exactly matches the configured first-party resource. This is not an Authorization Server and not a substitute for ChatGPT's interactive OAuth flow.
4. **Credential redirect rule:** authenticated MCP requests must use the repository's existing same-origin SSRF-safe redirect walker so a public endpoint cannot redirect the Bearer token into another origin or private network.
5. **Strict first-party modern adapter:** the configured public relay is accessed with a small first-party MCP `2026-07-28` adapter implementing only `server/discover`, `tools/list`, and `tools/call`. This avoids both weakening the Rust relay and depending on SDK-v1 session/SSE behavior for the canonical resource.
6. **Protocol package migration:** migration of generic outbound integrations from `@modelcontextprotocol/sdk` v1 to the split v2 client package remains a separate dependency slice because the legacy inbound Nuxt MCP endpoint still imports v1 server APIs. It must include a real lockfile update and local verification rather than a hand-edited dependency declaration. Once that migration is safe, the first-party adapter should be replaced by the official v2 client.
7. **Live evidence:** neither a static source check nor a Protected Resource Metadata curl is sufficient to claim ChatGPT interoperability. Final acceptance requires a real ChatGPT developer-mode connection, OAuth completion, tool discovery, and tool call.

## External facts rechecked

At implementation start, official MCP documentation identifies `2026-07-28` as the modern stateless HTTP revision, requires OAuth Protected Resource Metadata for protected HTTP resources, treats the MCP server as the Resource Server, and prefers Client ID Metadata Documents while retaining compatibility mechanisms such as DCR where supported. Current OpenAI MCP documentation requires a stable public HTTPS MCP endpoint for production integrations and supports OAuth 2.1/PKCE with MCP resource binding for authenticated ChatGPT connections.

These facts must be rechecked again if implementation resumes after the external specifications or OpenAI connection flow changes.
