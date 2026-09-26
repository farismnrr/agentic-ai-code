# SSO Auth Architecture

The service follows Clean Architecture boundaries and SOLID design by default.

## Backend layers

- `interfaces/` owns HTTP transport and request/response adaptation.
- `application/` owns authentication, connected-app registration, and trusted connection-handoff use cases and ports.
- `domain/` owns framework-independent authentication and connected-app concepts.
- `infrastructure/` owns environment, GitHub OAuth adapters, signing codecs, persistence, and external systems.
- `bootstrap/` is the composition root and wires concrete adapters.

Dependency direction points inward. Domain and application code must not depend on Axum, Tokio, Reqwest, SQL clients, or frontend concerns.

## Connected-app registry

Non-secret app registration belongs to the authenticated SSO dashboard rather than deployment environment variables.

Each registered app owns:

- client ID, which is also the signed assertion audience
- display name and description
- trusted callback URL
- enabled/disabled state
- short assertion TTL

The current persistence adapter stores registrations as JSON at `/app-data/connected-apps.json`. Compose mounts a named volume at `/app-data`, so app registration survives container recreation. `CONNECTED_APPS_PATH` is only an optional infrastructure override.

Secrets remain server configuration. `RELAY_ASSERTION_SECRET` is still required by SSO and Relay for the current HMAC handoff implementation.

## Relay connection handoff

A Relay connection starts with an opaque `connection_state` plus a registered `client_id`. SSO resolves that client ID from the registry before GitHub OAuth begins and again before the callback is completed.

The normal GitHub OAuth and allowlist flow remains the source of authenticated identity. SSO preserves the client ID and Relay state in HttpOnly, SameSite=Lax flow cookies while GitHub OAuth runs.

After OAuth succeeds and the allowlist accepts the user, `ConnectionService` reloads the enabled app registration and asks the `ConnectionAssertionIssuer` port for a short-lived assertion. The assertion contains the stable GitHub subject, login, registered client ID as audience, Relay state, issue time, and expiry.

The redirect destination is always the callback URL stored in the server-side app registry. Request input cannot replace it.

## Dashboard security

Connected-app registry APIs require a valid SSO session. In the current single-admin model, users admitted by the existing GitHub allowlist can manage the registry.

The dashboard never reads or writes the assertion signing secret.

## Frontend layers

- `frontend/app/` owns frontend composition.
- `frontend/features/` owns feature-specific UI and state as features appear.
- `frontend/shared/components/` owns reusable presentation components.
- `frontend/styles/` owns global Tailwind CSS and DaisyUI setup.

Prefer DaisyUI primitives and Tailwind utilities over hand-written component styling. Extract reusable components when a pattern has a real second use.

## Engineering rules

- Single responsibility: one reason to change per module/component.
- Open/closed: add adapters and use cases without rewriting unrelated layers.
- Liskov substitution: implementations must honor their port contracts.
- Interface segregation: keep ports narrow and use-case specific.
- Dependency inversion: application logic depends on ports, not concrete integrations.
- DRY: centralize repeated domain, validation, mapping, and presentation logic.
- Avoid speculative abstractions; reuse must come from a concrete repeated concern.
