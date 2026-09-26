# SSO Auth Architecture

The service follows Clean Architecture boundaries and SOLID design by default.

## Backend layers

- `interfaces/` owns HTTP transport and request/response adaptation.
- `application/` owns authentication and trusted connection-handoff use cases and ports.
- `domain/` owns framework-independent authentication concepts.
- `infrastructure/` owns environment, GitHub OAuth adapters, signing codecs, persistence, and external systems.
- `bootstrap/` is the composition root and wires concrete adapters.

Dependency direction points inward. Domain and application code must not depend on Axum, Tokio, Reqwest, SQL clients, or frontend concerns.

## Relay connection handoff

The normal GitHub OAuth and allowlist flow remains the source of authenticated identity. A Relay connection begins only when `/auth/github` receives a valid opaque `connection_state`.

SSO preserves that state in an HttpOnly, SameSite=Lax flow cookie while GitHub OAuth runs. After OAuth succeeds and the allowlist accepts the user, `ConnectionService` asks the `ConnectionAssertionIssuer` port for a short-lived signed assertion.

The assertion contains a stable GitHub subject plus login, issuer, audience, Relay state, issue time, and expiry. The redirect destination is the configuration-owned `RELAY_CALLBACK_URL`; request input cannot replace it.

The shared assertion secret is separate from `SESSION_SECRET`. Relay is responsible for signature, issuer, audience, expiry, and pending-state verification before it considers a connection established.

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
