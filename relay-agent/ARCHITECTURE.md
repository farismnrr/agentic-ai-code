# Relay Agent Architecture

The Relay service follows Clean Architecture and keeps authentication transport separate from trusted identity verification.

## Layers

- `interfaces/` owns HTTP routing, query parsing, redirects, status codes, and response headers.
- `application/` owns the auth-start and auth-callback use cases plus narrow ports.
- `domain/` owns framework-independent identity concepts produced only after trusted verification.
- `infrastructure/` owns environment config, SSO URL construction, and concrete verifier adapters.
- `bootstrap/` is the composition root and wires concrete adapters into the application layer.

Dependency direction points inward. `domain/` and `application/` must not depend on Axum, Tokio, URL parsing, crypto libraries, persistence, or HTTP details.

## Current authentication boundary

`GET /auth/login` asks the auth-start use case for the configured SSO login URL. The infrastructure adapter builds that URL from `SSO_BASE_URL` and a callback derived from `RELAY_PUBLIC_URL`; callers cannot supply an arbitrary return target.

`GET /auth/callback` accepts an opaque `assertion` input and passes it to `SignedAssertionVerifier`. The current concrete verifier deliberately returns `VerificationNotImplemented`, so the callback never treats raw query input as an authenticated identity and never creates a Relay session.

The current SSO route accepts the Relay-generated `return_to` query parameter syntactically but does not yet consume it or issue a Relay assertion. That cross-service behavior is intentionally deferred.

## Next authentication phase

The next phase should replace only the verifier adapter and extend the callback use case with session issuance after successful verification:

1. verify signature;
2. validate issuer;
3. validate audience;
4. validate expiry;
5. extract a verified principal;
6. create the Relay-local session.

Authentication and authorization remain distinct. The SSO GitHub allowlist controls who may obtain an SSO-authenticated session; Relay must still authenticate the signed handoff before trusting a principal, and any future Relay authorization policy belongs in its own application boundary.
