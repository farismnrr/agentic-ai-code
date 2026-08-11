# external MCP client Native MCP Integration Insights

This memory serves as a technical foundation for the Tech Lead (TL) to create a future plan regarding integrating the Rust Relay Agent natively with external MCP client's new MCP Custom Tools feature.

## Current Limitations
Our current Rust Relay Agent (Plan 028) was designed for legacy compatibility and strict local isolation:
1. **Transport Mismatch**: The relay agent currently serves stateless JSON-RPC over HTTP (`POST /mcp`). external MCP client's native MCP integration strictly requires the standard **MCP SSE (Server-Sent Events) Transport** (a long-lived event stream).
2. **OAuth Discovery & Config**: external MCP client actively hits `/.well-known/oauth-authorization-server` to discover OAuth endpoints (Auth URL, Token URL, etc.). While our agent exposes this route, it currently returns empty values because it acts purely as a Resource Server validating JWTs, not an Authorization Server.
3. **Public Exposure**: Our current access policy strictly blocks non-local `Origin` and `Host` headers unless a JWT is provided. Running via Cloudflare Tunnel without a properly configured OAuth flow results in external MCP client requests being dropped at the middleware layer.

## Architectural Requirements for external MCP client

To support external MCP client natively, a future plan must implement the following:

### 1. Standard MCP SSE Transport
- Implement a `GET /sse` route that establishes a persistent `axum::response::sse::EventStream`.
- Generate a unique Session ID for the SSE connection.
- Return the session-specific message endpoint to the client (e.g., `POST /message?session_id=...`).
- Implement the `POST /message` route to receive JSON-RPC calls, execute them using the existing `mcp.rs` logic, and push the JSON-RPC responses back to the corresponding SSE stream using an internal channel (`tokio::sync::mpsc`).

### 2. OAuth 2.0 Integration & Discovery
external MCP client requires a standard OAuth 2.0 flow to authenticate before interacting with the MCP server over the internet.
- **Provider Choice**: Integrate an external identity provider (e.g., Auth0, Clerk, or Supabase Auth) to handle the complex OAuth Authorization Code flow, rather than building an Authorization Server from scratch in Rust.
- **Discovery Endpoint**: Update `handle_well_known_oauth` to return the real `authorization_endpoint`, `token_endpoint`, and `registration_endpoint` of the chosen IdP.
- **Client Configuration**: external MCP client will act as the OAuth Client. The IdP must be configured with external MCP client's callback URL (e.g., `https://external-mcp.com/connector/oauth/...`).
- **Resource Server Validation**: The Relay Agent remains the Resource Server. It will validate the JWT Access Token sent by external MCP client using the IdP's JWKS (already partially implemented in `security.rs`).

### 3. Security Considerations (Best Practices)
- **Scope Limitations**: The MCP server should define explicit OAuth scopes for dangerous tools (e.g., `terminal_exec`). external MCP client allows defining base scopes and action-level scopes. 
- **User Segregation**: If exposed via Cloudflare Tunnel, the Relay Agent must enforce that the subject (`sub` claim) in the JWT matches the exact developer's identity. Otherwise, any external MCP client user with the URL could theoretically authenticate (if DCR is enabled) and execute commands.
- **Rate Limiting & Timeouts**: Ensure aggressive rate limiting on the `/message` and `/sse` endpoints to prevent DoS attacks through the tunnel.

## Recommended Phases for the TL
1. **Phase 1: SSE Transport**: Refactor `transport.rs` to support both legacy `POST /mcp` and the new `GET /sse` + `POST /message` pattern. Test locally with Cursor or external MCP client Desktop (which also support SSE).
2. **Phase 2: OAuth Setup**: Provision an Auth0/Clerk tenant, configure the external MCP client callback, and wire the metadata into the Relay Agent's `.toml` config.
3. **Phase 3: Integration & Scopes**: Connect external MCP client, verify the OAuth handshake, and implement scope-based tool gating in `mcp.rs`.
