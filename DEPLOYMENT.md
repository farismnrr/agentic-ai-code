# Operator deployment guide

This guide documents the single-owner Linux deployment used for the local `ai-tools` MCP relay, including release builds, the systemd user service, narrow host credential reuse, and delegated-agent network access.

## Build and install the relay binary

From the repository root:

```fish
pnpm --filter @ai-code/rust-tools build
mkdir -p ~/.local/bin
install -m 0755 target/release/ai-tools ~/.local/bin/ai-tools
~/.local/bin/ai-tools --version
```

Do not assume a service restart picked up new source code. The installed binary must be replaced first, and its reported version must match the repository release you intend to run.

The deployed service currently expects:

```text
AI_TOOLS_BIN=/home/<owner>/.local/bin/ai-tools
```

If a different installation path is used, update the systemd environment override to match it before restarting the relay.

## systemd user service

The service is managed as a user unit:

```text
ai-tools-relay.service
```

Operator overrides belong under:

```text
~/.config/systemd/user/ai-tools-relay.service.d/
```

Keep repository-managed startup behavior separate from host-local deployment overrides. A typical single-owner development override is:

```fish
mkdir -p ~/.config/systemd/user/ai-tools-relay.service.d

printf '%s\n' \
'[Service]' \
'Environment=AI_TOOLS_BIN=/home/<owner>/.local/bin/ai-tools' \
'Environment=RELAY_ALLOW_HOST_GITHUB_AUTH=true' \
'Environment=RELAY_ALLOW_TERMINAL_NETWORK=true' \
'Environment=RELAY_ALLOW_AGENT_NETWORK=true' \
> ~/.config/systemd/user/ai-tools-relay.service.d/local-development.conf
```

Replace `<owner>` with the local account name.

The three capability flags are independent:

- `RELAY_ALLOW_HOST_GITHUB_AUTH=true` lets ordinary terminal sandboxes reuse the owner's existing GitHub CLI and Git user configuration through narrow read-only mounts. It does not unmask the whole home directory.
- `RELAY_ALLOW_TERMINAL_NETWORK=true` permits network access for ordinary sandboxed terminal commands.
- `RELAY_ALLOW_AGENT_NETWORK=true` permits network access for delegated coding CLI processes. Terminal network permission alone does not grant delegated-agent network access.

Only enable these flags on a trusted single-owner development relay. Generic credential stores, SSH keys, cloud credentials, and unrelated protected paths remain outside the intended credential bridge.

## Reload and restart

After changing the installed binary or any systemd drop-in:

```fish
systemctl --user daemon-reload
systemctl --user restart ai-tools-relay.service
systemctl --user status ai-tools-relay.service --no-pager
```

A successful `restart` only proves that systemd started the configured process. It does not prove that the intended binary path or environment was selected.

## Verify the effective deployment

Inspect the final service configuration:

```fish
systemctl --user show ai-tools-relay.service \
  -p Environment \
  -p ExecStart
```

Verify all of the following before debugging MCP behavior:

1. `AI_TOOLS_BIN` points at the binary you just installed.
2. `ExecStart` points at the intended reviewed relay launcher.
3. Required capability flags appear in the effective `Environment` value.
4. The installed binary reports the expected version:

```fish
~/.local/bin/ai-tools --version
```

If the source tree is newer than the installed binary, rebuild and reinstall before restarting again.

## Recommended update sequence

For an existing deployment, use this order:

```fish
# 1. Update source and select the intended branch/commit.
git status --short --branch

# 2. Build the release binary.
pnpm --filter @ai-code/rust-tools build

# 3. Replace the exact binary selected by AI_TOOLS_BIN.
install -m 0755 target/release/ai-tools ~/.local/bin/ai-tools
~/.local/bin/ai-tools --version

# 4. Reload host-local systemd configuration and restart.
systemctl --user daemon-reload
systemctl --user restart ai-tools-relay.service

# 5. Confirm the effective process configuration.
systemctl --user show ai-tools-relay.service -p Environment -p ExecStart
systemctl --user status ai-tools-relay.service --no-pager
```

This sequence avoids the common failure mode where the repository contains a fix but the relay is still running an older installed binary.

## Post-restart smoke checks

After restart, validate through the same MCP execution paths used by clients rather than relying only on the host shell. Useful checks include:

- host CLI authentication is visible from an ordinary terminal tool when the credential bridge is enabled;
- network-capable terminal commands work only when terminal network access is enabled;
- delegated providers are advertised only when their executable and local session are actually usable;
- delegated requests can reach their service only when delegated-agent network access is enabled; and
- the live provider schema changes after login/logout only after the relay is restarted, because capability discovery runs at startup.

Keep these smoke checks read-only whenever possible.