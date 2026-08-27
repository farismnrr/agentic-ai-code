# Troubleshooting

## `pnpm install` aborts because there is no TTY

In a non-interactive/automation environment, pnpm may refuse to remove/recreate `node_modules` without confirmation.

Use the CI mode when appropriate:

```bash
CI=true pnpm install
```

Do not use this as a way to ignore a real dependency/install error; it only resolves the non-interactive confirmation requirement.

## Nuxt works in dev but fails after branch/file changes

Long-lived Nuxt dev watchers can retain stale generated/module state. For final verification:

```bash
pnpm build
pnpm preview
```

If generated types look stale, `pnpm install`/postinstall and `nuxt prepare` are part of the repository's normal setup/gates.

## Database works on host but not in Docker Compose

Inside a container, `localhost` refers to that container. Set `NUXT_DATABASE_URL_DOCKER` to a connection reachable from the container, typically through `host.docker.internal` for the reference compose topology.

## Workspace commands fail in the app container

`NUXT_WORKSPACES_ROOT` must be mounted into the container at the same absolute path expected by stored workspace paths. Check the volume in `docker-compose.yml`.

## Relay refuses to start

Check:

- OS is Linux;
- Bubblewrap (`bwrap`) is installed;
- process is not running as root;
- `--execution-root` exists and is a safe user-owned path;
- `--dir` exists under the intended scope;
- remote issuer/audience are canonical HTTPS URLs;
- `--trusted-proxy` has an explicit trusted CIDR.

Run:

```bash
ai-tools relay --help
```

for the exact current CLI surface.

## Relay command cannot find `node`, `pnpm`, or `cargo`

The relay intentionally does not inherit arbitrary host PATH. Add only the reviewed user-owned toolchain directories with `--toolchain-path` or `RELAY_TOOLCHAIN_PATH`.

Do not fix this with login-shell startup files that reintroduce the full host environment.

## Relay cannot access `.ssh`, Docker, or cloud credential directories

The owner-home sandbox masks common credential stores by design. This is expected.

Do not unmask credentials just to make a tool call convenient. Use an explicit safer workflow outside the relay when host credentials are genuinely required.

## `docker` says the daemon/socket is unavailable from MCP

Expected. The relay intentionally does not expose `/var/run/docker.sock`.

Run Docker-dependent operations—such as `pnpm release:publish`—from a trusted host shell with Docker configured. Do not use sudo/socket mounts/privileged-container tricks to bypass the relay boundary.

## Public MCP `/health` works but a client cannot connect

A healthy tunnel is only the first layer. Check in order:

1. `/.well-known/oauth-protected-resource/mcp` returns the expected resource and Authorization Server.
2. unauthenticated `server/discover` returns HTTP 401 with a Bearer challenge and `resource_metadata`.
3. OIDC discovery advertises the exact issuer configured on the relay.
4. JWKS is reachable.
5. issued token audience contains the exact `https://.../mcp` resource.
6. token scope contains `relay.coding`.
7. token `sub` matches `OAUTH_OWNER_SUBJECT`.
8. the callback URI supplied by the current client is allowlisted exactly.

Use:

```bash
REMOTE_MCP_URL='https://mcp.example.com/mcp' ops/remote-mcp/public-smoke.sh
```

before debugging client UI behavior.

## A client connects but shows an old tool catalog

The client may have cached/discovered an older MCP catalog. Refresh or recreate
the connection and test from a fresh session before concluding the server is
stale. The Full profile exposes the complete reviewed catalog; Primary is an
intentional reduced profile. Confirm the deployed `RELAY_TOOL_PROFILE` before
troubleshooting client-side visibility.

## Authenticated public smoke fails with 401/403

401 usually means the token is absent/invalid or issuer/audience/signature/time validation failed. 403 generally means authorization policy such as `relay.coding` scope or owner binding failed.

Avoid dumping the raw token into logs. Inspect bounded server diagnostics and Authorization Server configuration instead.

## Hosted Nuxt MCP test fails for another user

That can be the expected ownership guard. The private first-party relay token is only attached when both the URL and authoritative MCP row owner match `NUXT_REMOTE_MCP_URL` and `NUXT_REMOTE_MCP_OWNER_USER_ID`.

A second user creating a row with the same URL must not inherit the owner's credential.

## Release stays in draft

The release publisher intentionally publishes the release record only after
the OCI web image succeeds. Verify Docker is available and rerun from clean
tagged `main`:

```bash
CI=true pnpm release:publish vX.Y.Z
```

Do not manually publish the draft first if the intended release contract
requires web + CLI to ship atomically.
