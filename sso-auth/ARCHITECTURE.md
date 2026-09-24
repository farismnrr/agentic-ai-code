# SSO Auth Architecture

The service follows Clean Architecture boundaries and SOLID design by default.

## Backend layers

- `interfaces/` owns HTTP transport and request/response adaptation.
- `application/` owns authentication use cases and ports.
- `domain/` owns framework-independent authentication concepts.
- `infrastructure/` owns environment, GitHub OAuth adapters, persistence, and external systems.
- `bootstrap/` is the composition root and wires concrete adapters.

Dependency direction points inward. Domain and application code must not depend on Axum, Tokio, Reqwest, SQL clients, or frontend concerns.

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
