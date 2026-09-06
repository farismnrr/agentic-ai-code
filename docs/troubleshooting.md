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

The relay intentionally does not inherit arbitrary host PATH. Common owner runtimes are discovered from safe profile directories automatically; add other reviewed toolchain directories with `--toolchain-path` or `RELAY_TOOLCHAIN_PATH`.

Do not fix this with login-shell startup files that reintroduce the full host environment.

## Relay cannot access `.ssh`, Docker, or cloud credential directories

For broad terminal work, configure both `--dir "$HOME"` and
`--execution-root "$HOME"`; a home ceiling alone does not authorize sibling
projects. Credential and privilege boundaries still apply. A protected-path
discovery failure means the sandbox did not start: check for protected
symlinks, inaccessible directories or a tree above 500,000 entries. Use
narrower authorized roots instead of disabling masking or skipping visible
cache/build directories.

`systemctl --user` failing to connect to a bus is expected in this profile.
Host user-service control and journal mounts are not implicitly exposed by
HOME access. Do not mount a host D-Bus socket to work around this boundary.

The owner-home sandbox masks common credential stores by design. This is expected.

Do not unmask credentials just to make a tool call convenient. Use an explicit safer workflow outside the relay when host credentials are genuinely required.

## `docker` says the daemon/socket is unavailable from MCP

Expected. The relay intentionally does not expose `/var/run/docker.sock`.

Run Docker-dependent operations—such as `pnpm release:publish`—from a trusted host shell with Docker configured. Do not use sudo/socket mounts/privileged-container tricks to bypass the relay boundary.

## `ls /etc` works or commands can read `/etc/resolv.conf`

Expected behavior. The Bubblewrap sandbox provides system runtime mounts (`/usr`, `/lib`, `/etc`, `/bin`, `/sbin`) as read-only. This is required so compilers, runtimes, dynamic linkers, and system libraries can read DNS configuration (`/etc/resolv.conf`) and TLS root certificates (`/etc/ssl/certs`).

Attempting to modify or create files in `/etc` fails immediately with `Read-only file system`. Host user secrets, `/tmp`, `/proc`, and `/dev` remain isolated.

## Terminal commands fail with `Network is unreachable` or `curl: (7) Couldn't connect`

Expected default behavior. The terminal sandbox executes with an unshared network namespace (`--unshare-net`) by default. Subprocesses cannot connect to external hosts or local host ports.

If a trusted workflow requires network access (e.g. running `pnpm install`, `cargo build`, or downloading dependencies), set `RELAY_ALLOW_TERMINAL_NETWORK=true` or pass `--allow-terminal-network` to the relay. Dedicated HTTP tools (`http_fetch`, `web_search`) remain available independently.

## Local Git, node, or python work in terminal even though dedicated MCP wrappers are absent

Expected behavior. Local version control operations, script execution, builds, and test runs are intended terminal fallbacks. Removing high-level dedicated MCP wrappers from the public catalog does not ban standard non-privilege-escalating developer CLI tools from running inside the terminal sandbox. Only privilege brokers (`sudo`, `su`), SSH clients (`ssh`, `scp`), and dangerous daemons are blocked.

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
