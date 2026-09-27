# Environment Verification Report

## 1. Scope

Compared the whitelisted variables in the laptop files:

- `sso-auth/.env`
- `relay-agent/.env`

against their checked-in examples, application configuration, Docker Compose files, and `contracts/chatgpt-discovery.json`. On Orange Pi `100.89.159.121`, located and compared only:

- `/home/farismnrr/Documents/masih-awam/sso-auth/.env`
- `/home/farismnrr/Documents/masih-awam/relay-agent/.env`

Secret values were read only in memory to determine `SET`/`EMPTY`/`MISSING` and perform the requested Relay assertion-secret pair comparisons. No secret values or hashes are included here; no `.env` file or running service was changed.

The current service ports are laptop application ports 3000/3100. Orange Pi ports 5000/5001 are tunnel/ingress endpoints, not the expected laptop application `PORT` values.

## 2. Repository Expected Configuration

### SSO application

`sso-auth/src/infrastructure/config/app_config.rs` consumes:

| variable | requirement / default | purpose |
|---|---|---|
| `PORT` | Optional; default `3000` | Local application listen port |
| `SSO_BASE_URL` | Required absolute HTTP(S) URL | Public issuer/base URL; `https` enables secure cookies |
| `MCP_RESOURCE_URL` | Optional; default `https://relay.farismnrr.com/mcp` | MCP access-token resource/audience |
| `GITHUB_CLIENT_ID` | Required | GitHub OAuth client credential |
| `GITHUB_CLIENT_SECRET` | Required | GitHub OAuth client credential |
| `GITHUB_CALLBACK_URL` | Required | Redirect URI sent to GitHub |
| `GITHUB_AUTHORIZE_URL` | Optional; default `https://github.com/login/oauth/authorize` | GitHub authorization endpoint |
| `GITHUB_TOKEN_URL` | Optional; default `https://github.com/login/oauth/access_token` | GitHub token endpoint |
| `GITHUB_API_URL` | Optional; default `https://api.github.com/` | GitHub API base |
| `ALLOWED_GITHUB_USER_IDS` | Required | SSO allowlist |
| `SESSION_SECRET` | Required | Session signing secret |
| `SESSION_TTL_SECONDS` | Optional; default `604800` | Session lifetime |
| `RELAY_ASSERTION_SECRET` | Required | Shared Relay assertion/token signing secret |

The checked-in SSO example uses the canonical public SSO base, callback and MCP resource, and `PORT=3000`. Its GitHub endpoint overrides and session TTL equal the source defaults.

`AGENTATION_ENABLED` is documented as a build/CI development-inspector setting, not read by the SSO backend `AppConfig`. The frontend reads `VITE_AGENTATION_ENABLED` at build time; the Dockerfile defaults that build argument to `false`. The checked-in example's `AGENTATION_ENABLED=false` is the disabled setting, but a value in a runtime `env_file` does not by itself change an already-built frontend bundle.

The SSO E2E smoke harness also reads `E2E_BASE_URL` (default `http://127.0.0.1:13000`), `E2E_ARTIFACT_DIR` (default `/artifacts`), and `E2E_OAUTH_PORT` (default `4400`). These are test-harness inputs, not production service configuration.

### Relay application

`relay-agent/src/infrastructure/config/app_config.rs` consumes:

| variable | requirement / default | purpose |
|---|---|---|
| `PORT` | Optional; default `3100` | Local application listen port |
| `SSO_BASE_URL` | Required absolute HTTP(S) URL | SSO connection redirect base and expected assertion issuer |
| `RELAY_PUBLIC_URL` | Required absolute HTTP(S) URL | Public Relay identity used to construct discovery URLs, MCP resource, and protected-resource metadata |
| `RELAY_ASSERTION_SECRET` | Required | Must match SSO's assertion signing secret |

The Relay bootstrap derives `/mcp`, protected-resource metadata, connection-start, and status URLs from `RELAY_PUBLIC_URL`; it also configures the token verifier with the derived MCP resource and `SSO_BASE_URL` issuer.

### Compose and public contract

- `docker-compose.fast.yml` publishes the fast services on host ports 3000 and 3100, matching their application ports. These Compose host bindings do not change the required `PORT` values.
- `docker-compose.latest.yml` is SSO-only and maps host loopback port 5000 to container port 3000. This is a deployment/ingress mapping, not the canonical laptop `PORT`.
- The shared contract specifies issuer `https://sso.farismnrr.com`, resource `https://relay.farismnrr.com/mcp`, protected-resource metadata `https://relay.farismnrr.com/.well-known/oauth-protected-resource/mcp`, and ChatGPT's CIMD client ID and redirect URI.

## 3. Laptop Environment

### SSO (`sso-auth/.env`)

| variable | actual value |
|---|---|
| `SSO_BASE_URL` | `http://localhost:3000` |
| `MCP_RESOURCE_URL` | MISSING; source fallback is `https://relay.farismnrr.com/mcp` |
| `GITHUB_CALLBACK_URL` | `http://localhost:3000/auth/github/callback` |
| `GITHUB_AUTHORIZE_URL` | `https://github.com/login/oauth/authorize` |
| `GITHUB_TOKEN_URL` | `https://github.com/login/oauth/access_token` |
| `GITHUB_API_URL` | `https://api.github.com/` |
| `ALLOWED_GITHUB_USER_IDS` | `120432426` |
| `SESSION_TTL_SECONDS` | `604800` |
| `AGENTATION_ENABLED` | `true` |
| `PORT` | `3000` |

Secret status only: `GITHUB_CLIENT_ID=SET`, `GITHUB_CLIENT_SECRET=SET`, `SESSION_SECRET=SET`, `RELAY_ASSERTION_SECRET=SET`.

### Relay (`relay-agent/.env`)

| variable | actual value |
|---|---|
| `RELAY_PUBLIC_URL` | `http://localhost:3100` |
| `SSO_BASE_URL` | `http://localhost:3000` |
| `PORT` | `3100` |

Secret status only: `RELAY_ASSERTION_SECRET=SET`. Its value matches the value in laptop SSO.

## 4. Old Orange Pi Environment

The old files remain at `/home/farismnrr/Documents/masih-awam/sso-auth/.env` and `/home/farismnrr/Documents/masih-awam/relay-agent/.env`.

### Old SSO

| variable | actual value |
|---|---|
| `SSO_BASE_URL` | `https://sso.farismnrr.com` |
| `MCP_RESOURCE_URL` | MISSING; source fallback is `https://relay.farismnrr.com/mcp` |
| `GITHUB_CALLBACK_URL` | `https://sso.farismnrr.com/auth/github/callback` |
| `GITHUB_AUTHORIZE_URL` | `https://github.com/login/oauth/authorize` |
| `GITHUB_TOKEN_URL` | `https://github.com/login/oauth/access_token` |
| `GITHUB_API_URL` | `https://api.github.com/` |
| `ALLOWED_GITHUB_USER_IDS` | `120432426` |
| `SESSION_TTL_SECONDS` | `604800` |
| `AGENTATION_ENABLED` | `false` |
| `PORT` | `5000` |

Secret status only: `GITHUB_CLIENT_ID=SET`, `GITHUB_CLIENT_SECRET=SET`, `SESSION_SECRET=SET`, `RELAY_ASSERTION_SECRET=SET`.

### Old Relay

| variable | actual value |
|---|---|
| `RELAY_PUBLIC_URL` | `https://relay.farismnrr.com` |
| `SSO_BASE_URL` | `https://sso.farismnrr.com` |
| `PORT` | `3100` |

Secret status only: `RELAY_ASSERTION_SECRET=SET`. Its value matches old SSO's value.

The old SSO `PORT=5000` belongs to that old Orange Pi application configuration. It is not the laptop application port. The old Orange Pi host ingress ports 5000/5001 are not canonical laptop `PORT` values.

## 5. Comparison Matrix

For `MCP_RESOURCE_URL`, the matrix shows the raw environment-file value and its effective source fallback. Status reflects the effective configuration where a source default applies. For secret rows, values are presence/match states only.

| service | variable | laptop | old Orange Pi | repo/example | expected | status |
|---|---|---|---|---|---|---|
| SSO | `SSO_BASE_URL` | `http://localhost:3000` | `https://sso.farismnrr.com` | `https://sso.farismnrr.com` | `https://sso.farismnrr.com` | STALE |
| SSO | `MCP_RESOURCE_URL` | MISSING; effective fallback is canonical URL | MISSING; effective fallback is canonical URL | `https://relay.farismnrr.com/mcp` | `https://relay.farismnrr.com/mcp` | MATCH |
| SSO | `GITHUB_CALLBACK_URL` | `http://localhost:3000/auth/github/callback` | `https://sso.farismnrr.com/auth/github/callback` | `https://sso.farismnrr.com/auth/github/callback` | `https://sso.farismnrr.com/auth/github/callback` | STALE |
| SSO | `GITHUB_AUTHORIZE_URL` | `https://github.com/login/oauth/authorize` | Same | Same; source default | `https://github.com/login/oauth/authorize` | MATCH |
| SSO | `GITHUB_TOKEN_URL` | `https://github.com/login/oauth/access_token` | Same | Same; source default | `https://github.com/login/oauth/access_token` | MATCH |
| SSO | `GITHUB_API_URL` | `https://api.github.com/` | Same | Same; source default | `https://api.github.com/` | MATCH |
| SSO | `ALLOWED_GITHUB_USER_IDS` | `120432426` | `120432426` | `120432426` | Existing allowlist | MATCH |
| SSO | `SESSION_TTL_SECONDS` | `604800` | `604800` | `604800`; source default | `604800` | MATCH |
| SSO | `AGENTATION_ENABLED` | `true` in runtime `.env` | `false` | `false` in example; frontend build flag is `VITE_AGENTATION_ENABLED` | Disabled build default is `false`; runtime `.env` value is not consumed by backend | STALE (inert runtime setting) |
| SSO | `PORT` | `3000` | `5000` | `3000`; source default | `3000` | DIFFERENT_BY_DESIGN (old Orange Pi app port) |
| SSO | `GITHUB_CLIENT_ID` | SET | SET | Blank placeholder | `<existing-secret>` | SECRET_SET |
| SSO | `GITHUB_CLIENT_SECRET` | SET | SET | Blank placeholder | `<existing-secret>` | SECRET_SET |
| SSO | `SESSION_SECRET` | SET | SET | Blank placeholder | `<existing-secret>` | SECRET_SET |
| SSO | `RELAY_ASSERTION_SECRET` | SET; matches laptop Relay | SET; matches old Relay | Blank placeholder | Same existing value in SSO and Relay | SECRET_MATCH |
| Relay | `RELAY_PUBLIC_URL` | `http://localhost:3100` | `https://relay.farismnrr.com` | `http://localhost:3100` | `https://relay.farismnrr.com` | STALE |
| Relay | `SSO_BASE_URL` | `http://localhost:3000` | `https://sso.farismnrr.com` | `https://sso.farismnrr.com` | `https://sso.farismnrr.com` | STALE |
| Relay | `PORT` | `3100` | `3100` | `3100`; source default | `3100` | MATCH |
| Relay | `RELAY_ASSERTION_SECRET` | SET; matches laptop SSO | SET; matches old SSO | Blank placeholder | Same existing value as SSO | SECRET_MATCH |

The old SSO `PORT=5000` differs from the laptop's `3000` by design. The new laptop process must use `3000`; Orange Pi 5000 is the SSH ingress/tunnel endpoint. Relay continues to listen on application port `3100`; Orange Pi 5001 is its ingress/tunnel endpoint.

## 6. Secret Consistency

No secret values or hashes are recorded. `MATCH` in the first three rows means both environments have the variable set; their values were not compared. For `RELAY_ASSERTION_SECRET`, values were compared only between SSO and Relay within each environment.

| variable | laptop | old Orange Pi | consistency |
|---|---|---|---|
| `GITHUB_CLIENT_ID` | SET | SET | MATCH |
| `GITHUB_CLIENT_SECRET` | SET | SET | MATCH |
| `SESSION_SECRET` | SET | SET | MATCH |
| `RELAY_ASSERTION_SECRET` | SET | SET | MATCH |

`RELAY_ASSERTION_SECRET` matches between laptop SSO and Relay, and separately between old Orange Pi SSO and Relay. Cross-environment secret equality was not tested.

## 7. Required Laptop Changes

### Required for canonical public identities

VARIABLE: `sso-auth/.env:SSO_BASE_URL`  
CURRENT: `http://localhost:3000`  
EXPECTED: `https://sso.farismnrr.com`  
REASON: The issuer and advertised OAuth authorization/token endpoint base must use the canonical SSO origin; HTTPS also makes SSO cookies secure.

VARIABLE: `sso-auth/.env:GITHUB_CALLBACK_URL`  
CURRENT: `http://localhost:3000/auth/github/callback`  
EXPECTED: `https://sso.farismnrr.com/auth/github/callback`  
REASON: SSO sends this redirect URI to GitHub; localhost is not the public registered callback.

VARIABLE: `relay-agent/.env:RELAY_PUBLIC_URL`  
CURRENT: `http://localhost:3100`  
EXPECTED: `https://relay.farismnrr.com`  
REASON: Relay derives its public MCP resource, protected-resource metadata, discovery, and connection URLs from this value.

VARIABLE: `relay-agent/.env:SSO_BASE_URL`  
CURRENT: `http://localhost:3000`  
EXPECTED: `https://sso.farismnrr.com`  
REASON: Relay uses this base for SSO connection redirects and as the expected assertion issuer.

`sso-auth/.env:MCP_RESOURCE_URL` is absent, but the source default is already the expected `https://relay.farismnrr.com/mcp`; an explicit value is optional, not required to correct the effective configuration.

### Development-only setting

`sso-auth/.env:AGENTATION_ENABLED=true` differs from the example's disabled value but is not a backend runtime setting. The SSO runtime image is configured through `VITE_AGENTATION_ENABLED` at build time, defaulting to `false`; changing the runtime `.env` value alone does not change the compiled UI. For disabled behavior, use `VITE_AGENTATION_ENABLED=false` as the build input. No runtime `.env` change for this key is required to restore the public URLs.

No secret synchronization is required based on presence or the requested shared-secret checks: all required secret entries are set, and the assertion secret matches within both SSO/Relay pairs. Secret values were not copied or recorded.

## 8. .env.example Drift

- **Relay example is stale:** `relay-agent/.env.example` sets `RELAY_PUBLIC_URL=http://localhost:3100`; the canonical expected value is `https://relay.farismnrr.com`. Relay uses this value to emit absolute discovery/resource URLs, so this is checked-in repository drift.
- The SSO example's `SSO_BASE_URL`, `GITHUB_CALLBACK_URL`, and `MCP_RESOURCE_URL` match the public contract.
- The SSO example's `PORT=3000` and Relay example's `PORT=3100` match application defaults and laptop ports.
- The SSO example's `AGENTATION_ENABLED=false` is a disabled/development setting; the frontend consumes the corresponding `VITE_AGENTATION_ENABLED` build-time input. It is not a backend runtime setting.

## 9. Runtime Impact

- **OAuth issuer and discovery:** SSO constructs OAuth metadata issuer and authorization/token endpoint URLs from `SSO_BASE_URL`. Laptop `http://localhost:3000` advertises a local HTTP issuer instead of `https://sso.farismnrr.com`.
- **GitHub callback:** The laptop callback points to localhost. SSO uses `GITHUB_CALLBACK_URL` in its GitHub OAuth request; it must be the canonical externally registered callback.
- **Cookie security:** SSO sets `cookie_secure` according to whether `SSO_BASE_URL` uses HTTPS. The laptop's HTTP localhost value makes that flag false; the canonical HTTPS value makes it true.
- **MCP resource audience:** SSO's absent `MCP_RESOURCE_URL` currently falls back to the canonical MCP resource. Relay, however, derives the expected resource from `RELAY_PUBLIC_URL`; the laptop's localhost Relay URL therefore makes Relay expect a different resource/audience from SSO's default.
- **Relay discovery and ChatGPT:** Relay constructs absolute MCP, protected-resource metadata, connection-start, and status URLs from `RELAY_PUBLIC_URL`. With localhost configured, these public discovery values identify the laptop's loopback instead of `relay.farismnrr.com`, conflicting with the ChatGPT discovery contract.
- **Relay-to-SSO handoff:** Relay uses `SSO_BASE_URL` both for SSO connection redirects and the expected issuer when verifying assertions. The localhost HTTP value diverges from the contract issuer.
- **Connected-app callback:** The SSO-to-Relay callback destination is stored in SSO's connected-app SQLite registry, not in these environment variables. This comparison does not inspect or change that stored registration; `RELAY_PUBLIC_URL` still controls Relay's public discovery and connection-start URLs.
- **Development inspector:** `AGENTATION_ENABLED` in the runtime `.env` does not prove the compiled frontend inspector state. The relevant value is the build-time `VITE_AGENTATION_ENABLED`.

## 10. Recommended Final Values

Non-secret runtime values:

```dotenv
# sso-auth/.env
SSO_BASE_URL=https://sso.farismnrr.com
MCP_RESOURCE_URL=https://relay.farismnrr.com/mcp
GITHUB_CALLBACK_URL=https://sso.farismnrr.com/auth/github/callback
GITHUB_AUTHORIZE_URL=https://github.com/login/oauth/authorize
GITHUB_TOKEN_URL=https://github.com/login/oauth/access_token
GITHUB_API_URL=https://api.github.com/
ALLOWED_GITHUB_USER_IDS=120432426
SESSION_TTL_SECONDS=604800
PORT=3000

# relay-agent/.env
RELAY_PUBLIC_URL=https://relay.farismnrr.com
SSO_BASE_URL=https://sso.farismnrr.com
PORT=3100
```

Optional UI build input (not a runtime `.env` setting):

```text
VITE_AGENTATION_ENABLED=false
```

Secret placeholders only; keep the existing values securely:

```dotenv
GITHUB_CLIENT_ID=<existing-secret>
GITHUB_CLIENT_SECRET=<existing-secret>
SESSION_SECRET=<existing-secret>
RELAY_ASSERTION_SECRET=<existing-secret>
```

## 11. Exact Facts For Next Agent

```text
LAPTOP_SSO_PORT=3000
LAPTOP_RELAY_PORT=3100

EXPECTED_SSO_BASE_URL=https://sso.farismnrr.com
EXPECTED_MCP_RESOURCE_URL=https://relay.farismnrr.com/mcp
EXPECTED_GITHUB_CALLBACK_URL=https://sso.farismnrr.com/auth/github/callback
EXPECTED_RELAY_PUBLIC_URL=https://relay.farismnrr.com
EXPECTED_RELAY_SSO_BASE_URL=https://sso.farismnrr.com

LAPTOP_SSO_ENV_STATUS=stale
LAPTOP_RELAY_ENV_STATUS=stale

RELAY_ASSERTION_SECRET_STATUS=MATCH

ENV_EXAMPLE_DRIFT=yes

CHANGES_REQUIRED=yes
```

## 12. Environment Remediation Result

```text
SSO_BASE_URL=https://sso.farismnrr.com
MCP_RESOURCE_URL=https://relay.farismnrr.com/mcp
GITHUB_CALLBACK_URL=https://sso.farismnrr.com/auth/github/callback
RELAY_PUBLIC_URL=https://relay.farismnrr.com
RELAY_SSO_BASE_URL=https://sso.farismnrr.com

SSO_PORT=3000
RELAY_PORT=3100

AGENTATION_ENABLED=false

SSO_LOCAL_VALIDATION=HTTP 200 health and authorization-server metadata; metadata fields canonical; generated GitHub redirect_uri canonical
RELAY_LOCAL_VALIDATION=HTTP 200 discovery and protected-resource metadata; connect/status URLs, resource, and authorization server canonical

SSO_TUNNEL_VALIDATION=HTTP 200 for Orange Pi 127.0.0.1:5000 health and authorization-server metadata
RELAY_TUNNEL_VALIDATION=HTTP 200 for Orange Pi 127.0.0.1:5001 discovery and protected-resource metadata

SSO_PUBLIC_VALIDATION=HTTP 200; issuer, authorization_endpoint, and token_endpoint canonical; no localhost or loopback URLs
RELAY_PUBLIC_VALIDATION=HTTP 200; discovery and protected-resource values canonical; no localhost or loopback URLs

LOCALHOST_PUBLIC_URLS_REMAINING=no
ENV_EXAMPLE_DRIFT_FIXED=yes

REMEDIATION_RESULT=success
```

The first Relay public curl immediately after recreation returned HTTP 502; subsequent fetch and curl validations for both Relay public discovery endpoints returned HTTP 200. No ingress configuration was changed.
