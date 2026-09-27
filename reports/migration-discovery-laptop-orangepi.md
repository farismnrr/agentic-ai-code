# Migration Discovery Report

## 1. Repository State

- **Branch:** `refactor/full-fe-be-relay`
- **Commit:** `db1c0333957a4f6a46f739b1861d6ad68ad7e45f`
- Read `AGENTS.md`, `CONTRIBUTING.md`, all three requested Compose files, both service architecture documents, `contracts/chatgpt-discovery.json`, and both `.env.example` files. No actual `.env` values were read.
- The base and `latest` Compose files bind SSO to loopback; the fast Compose file binds both services to `0.0.0.0`.
- SSO and Relay listen on container ports 3000 and 3100. Both application configurations bind to `0.0.0.0` inside their containers.
- SSO publishes OAuth metadata and endpoints; Relay publishes MCP, connection, and resource-discovery routes. Relay builds absolute public links from `RELAY_PUBLIC_URL`; SSO derives its issuer from `SSO_BASE_URL`.
- SSO’s connected-app callback URL is held in its SQLite registry, not in deployment environment configuration. Relay’s pending connection state is in memory and is lost on process restart.

Environment variable names found in the examples/configuration:

- SSO/public URLs: `SSO_BASE_URL`, `MCP_RESOURCE_URL`, `GITHUB_CALLBACK_URL`
- Relay/public URLs: `RELAY_PUBLIC_URL`, `SSO_BASE_URL`
- Ports: `PORT`
- Service bind address: no bind-address environment variable found; both services bind to `0.0.0.0`.
- OAuth resource and callbacks: `MCP_RESOURCE_URL`, `GITHUB_CALLBACK_URL`
- Allowed origins: no environment variable found; Relay checks the request `Origin` against its configured MCP resource origin and the ChatGPT origin.
- Tunnel/proxy settings: none found.
- OAuth issuer uses `SSO_BASE_URL`; no separate issuer variable was found.

## 2. Laptop

These are facts visible inside the current workstation environment, which is Ubuntu under WSL2; the physical host’s network configuration is not exposed here.

- **Hostname:** `thinkpad`
- **OS:** Ubuntu 24.04.5 LTS on WSL2, Linux kernel `6.18.33.2-microsoft-standard-WSL2`
- **Architecture:** x86_64
- **LAN-side address in this environment:** `10.10.1.36/24` on `eth2`
- **Default gateway:** `10.10.1.1`
- **Address assignment:** the interface reports a valid lifetime of `forever`; whether the physical laptop’s LAN address is DHCP/dynamic could not be determined from WSL.
- A separate `eth1` has `100.82.13.112/32` and a Tailscale-style IPv6 address. The exact software owning that interface was not established.
- **Docker:** client and server `29.8.1`; Compose `v5.5.1`.
- The laptop SSH port 22 was closed/refused on `10.10.1.36`. The Orange Pi route/reachability was not tested because its hostname did not resolve and its IP was not known.

## 3. Current Docker Services

Runtime inspection showed both services running from `:fast` images. Neither container reports a Docker health check.

| service | container | image | host bind | host port | container port | health |
|---|---|---|---|---:|---:|---|
| sso-auth | `agentic-ai-code-sso-auth-1` | `ghcr.io/farismnrr/agentic-ai-code-sso-auth:fast` | `0.0.0.0` | 3000 | 3000 | running; no Docker health check |
| relay-agent | `agentic-ai-code-relay-agent-1` | `ghcr.io/farismnrr/agentic-ai-code-relay-agent:fast` | `0.0.0.0` | 3100 | 3100 | running; no Docker health check |

Both containers use Docker network `agentic-ai-code_default` (bridge). Their internal IPs were `172.18.0.2` for SSO and `172.18.0.3` for Relay. The Compose service names, and thus the relevant internal Docker hostnames on that network, are `sso-auth` and `relay-agent`.

The running bindings match `docker-compose.fast.yml`; they differ from the loopback-only bindings in `docker-compose.yml` and `docker-compose.latest.yml`.

## 4. Orange Pi

Orange Pi inspection was **not available** from this environment:

- SSH config has an alias `orangepi` with user `farismnrr` and port 22, but the alias did not resolve.
- Ping and TCP/22 probes to that alias failed at name resolution. A BatchMode SSH attempt also failed because the hostname could not be resolved.
- No Orange Pi hostname, OS, architecture, LAN/public address, NAT status, SSH daemon state, forwarding settings, firewall state, listening ports, or installed services could therefore be confirmed.
- No evidence was collected that it runs SSO or Relay. The requested constraint that it must not run either service remains a requirement, not an observed fact.

Run these read-only commands **on the Orange Pi** to collect the missing host facts. The forwarding query prints only the named effective SSH settings, not private keys or credentials:

```sh
hostnamectl
uname -m
cat /etc/os-release
ip -brief address
ip route
curl -4 -sS --max-time 5 https://api.ipify.org; echo
sudo ss -lntup
sudo sshd -T | grep -Ei '^(allowtcpforwarding|gatewayports|permitopen|permitlisten|clientaliveinterval|clientalivecountmax) '
sudo ufw status
sudo nft list ruleset
sudo iptables -S
systemctl list-units --type=service --all --no-pager |
  grep -Ei 'nginx|caddy|traefik|cloudflared|frps|frpc|autossh|tailscale|zerotier|wireguard|tunnel|sso-auth|relay-agent'
```

Purpose: identify OS/addressing and public egress IP; compare the public IP with the router’s WAN address to assess NAT/CGNAT; inspect SSH policy, firewall rules, listeners, relevant services, and verify SSO/Relay are absent. To test laptop-to-Orange-Pi reachability, first obtain its LAN IP, then from the laptop run `ping -c 1 <ORANGE_PI_LAN_IP>` and `nc -vz -w 3 <ORANGE_PI_LAN_IP> 22`. To test Orange Pi-to-laptop routing, use the laptop’s actual reachable LAN/tailnet address and existing listener ports only; do not expose a new port.

## 5. DNS / Public Ingress

Public DNS answers were queried through Google Public DNS over HTTPS. Cloudflare nameservers are authoritative for the domain. DNS pointing at Cloudflare anycast addresses is consistent with Cloudflare proxying; the Relay HTTPS response independently confirms Cloudflare handling. The SSO record instead returns a `100.64.0.0/10` address, not Cloudflare anycast.

| hostname | record type | destination | Cloudflare/proxy | TLS likely terminates at | notes |
|---|---|---|---|---|---|
| `farismnrr.com` | A | `172.67.146.168`, `104.21.63.143` | Appears proxied through Cloudflare | Cloudflare edge likely; origin unknown | Also has Cloudflare IPv6 addresses `2606:4700:3030::6815:3f8f` and `2606:4700:3033::ac43:92a8`. |
| `farismnrr.com` | AAAA | Cloudflare IPv6 addresses above | Appears proxied through Cloudflare | Cloudflare edge likely; origin unknown | Authoritative DNS is Cloudflare. |
| `sso.farismnrr.com` | A | `100.89.159.121` | No evidence of Cloudflare proxying; answer is in shared CGNAT range | HTTPS response identifies `nginx/1.29.4`; exact TLS host/location unknown | No AAAA answer. This address is not generally routable from the public Internet. Current workstation can reach it, plausibly via its `100.x` overlay interface; that does not prove public reachability. |
| `sso.farismnrr.com` | AAAA | No answer | Unknown | Unknown | Public DNS query returned no AAAA answer. |
| `relay.farismnrr.com` | A | `172.67.146.168`, `104.21.63.143` | Appears proxied through Cloudflare | Cloudflare edge likely; origin unknown | HTTPS response includes `server: cloudflare` and `cf-cache-status: DYNAMIC`. |
| `relay.farismnrr.com` | AAAA | `2606:4700:3030::6815:3f8f`, `2606:4700:3033::ac43:92a8` | Appears proxied through Cloudflare | Cloudflare edge likely; origin unknown | HTTPS response was HTTP/2 through Cloudflare. |

CNAME queries returned no CNAME answers. Public DNS does not identify whether either record targets the Orange Pi; the SSO address is not an Orange Pi identity established by this discovery.

## 6. Current OAuth / MCP Public Contract

Canonical values below come from the checked-in contract and source configuration. Runtime `.env` values were not read, so their exact correspondence to these canonical values was not independently verified.

| purpose | current URL | source file | stability requirement | notes |
|---|---|---|---|---|
| OAuth issuer | `https://sso.farismnrr.com` | `contracts/chatgpt-discovery.json`; `sso-auth/.env.example`; `sso-auth/src/infrastructure/config/app_config.rs` | Must remain stable or issuer/resource/token validation contracts and clients must be updated together | Issuer derives from `SSO_BASE_URL`. |
| Authorization-server metadata | `https://sso.farismnrr.com/.well-known/oauth-authorization-server` | `sso-auth/src/interfaces/http/router.rs`; `sso-auth/src/interfaces/http/oauth.rs` | Preserve path and issuer URL, or update discovery clients/contracts | Metadata advertises authorization and token endpoints. |
| OAuth authorization endpoint | `https://sso.farismnrr.com/oauth/authorize` | `sso-auth/src/interfaces/http/oauth.rs` | Preserve URL or update metadata consumers | Advertised by authorization-server metadata. |
| OAuth token endpoint | `https://sso.farismnrr.com/oauth/token` | `sso-auth/src/interfaces/http/oauth.rs` | Preserve URL or update metadata consumers | Advertised by authorization-server metadata. |
| MCP resource | `https://relay.farismnrr.com/mcp` | `contracts/chatgpt-discovery.json`; `relay-agent/src/bootstrap/runner.rs`; `sso-auth/.env.example` | Must remain aligned with token audience/resource validation | Also configured in SSO as `MCP_RESOURCE_URL`. |
| MCP endpoint | `https://relay.farismnrr.com/mcp` | `contracts/chatgpt-discovery.json`; `relay-agent/src/interfaces/http/router.rs` | Must remain stable for configured clients or be updated in contract and discovery | Relay accepts POST at `/mcp`. |
| Protected-resource metadata (resource-specific) | `https://relay.farismnrr.com/.well-known/oauth-protected-resource/mcp` | `contracts/chatgpt-discovery.json`; `relay-agent/src/bootstrap/runner.rs`; `relay-agent/src/interfaces/http/router.rs` | Must match Relay’s challenge header and metadata routes | Advertised in unauthenticated MCP `WWW-Authenticate`. |
| Protected-resource metadata (root) | `https://relay.farismnrr.com/.well-known/oauth-protected-resource` | `relay-agent/src/interfaces/http/router.rs` | Preserve if clients rely on root discovery | Separate root metadata handler. |
| Relay discovery document | `https://relay.farismnrr.com/.well-known/relay.json` | `relay-agent/src/interfaces/http/router.rs`; `relay-agent/src/interfaces/http/discovery.rs` | Preserve or update discovery consumers | Returns connection URL and status URL template using configured `RELAY_PUBLIC_URL`. |
| Relay connection start | `https://relay.farismnrr.com/connections/start` | `relay-agent/src/interfaces/http/router.rs`; `relay-agent/src/bootstrap/runner.rs` | Preserve or update Relay discovery document | Redirects through SSO connect flow. |
| Relay connection callback | `https://relay.farismnrr.com/connections/callback` | `relay-agent/src/interfaces/http/router.rs`; SSO callback destination is stored in SSO SQLite per `sso-auth/ARCHITECTURE.md` | Registered callback URL must exactly match Relay’s externally reachable URL | The route is present; the actual SQLite-registered callback value was not inspected. |
| ChatGPT CIMD client ID | `https://chatgpt.com/oauth/client.json` | `contracts/chatgpt-discovery.json` | Must remain the client identifier used by SSO’s CIMD resolver | SSO supports client metadata documents. |
| ChatGPT OAuth redirect URI | `https://chatgpt.com/connector_platform_oauth_redirect` | `contracts/chatgpt-discovery.json` | Must match the CIMD client registration accepted by SSO | Not a repository-hosted endpoint. |
| GitHub OAuth callback | `https://sso.farismnrr.com/auth/github/callback` | `sso-auth/.env.example`; `sso-auth/src/interfaces/http/router.rs` | Must match the GitHub OAuth application callback configuration | Separate from the ChatGPT redirect URI. |

The repository’s Relay code constructs absolute URLs from `RELAY_PUBLIC_URL`, not the incoming `Host` header. Its MCP origin check accepts HTTPS `chatgpt.com` or the configured resource’s same origin. The inspected code did not show `X-Forwarded-*` handling.

## 7. Existing Network Path

```text
Observed local workstation environment:
  WSL2 Ubuntu (thinkpad)
    |
    +-- eth2: 10.10.1.36/24, default gateway 10.10.1.1
    |
    +-- Docker bridge: agentic-ai-code_default
          |-- sso-auth: 172.18.0.2:3000
          |-- relay-agent: 172.18.0.3:3100
          |
          +-- published on 0.0.0.0:3000 and 0.0.0.0:3100

Observed public hostname responses:
  sso.farismnrr.com -> 100.89.159.121 -> HTTPS nginx response
  relay.farismnrr.com -> Cloudflare anycast -> Cloudflare HTTPS response

Orange Pi position and any path between it, the laptop, and either public hostname:
  not established
```

## 8. Connectivity Results

| source | destination | protocol/port | result | notes |
|---|---|---|---|---|
| Laptop environment | `127.0.0.1:3000/` | HTTP | HTTP 200 | SSO root responds. |
| Laptop environment | `127.0.0.1:3000/health` | HTTP | HTTP 200 | SSO health route responds. |
| Laptop environment | `127.0.0.1:3000/.well-known/oauth-authorization-server` | HTTP | HTTP 200 | SSO metadata route responds. |
| Laptop environment | `127.0.0.1:3100/.well-known/relay.json` | HTTP | HTTP 200 | Relay discovery responds. |
| Laptop environment | `127.0.0.1:3100/.well-known/oauth-protected-resource/mcp` | HTTP | HTTP 200 | Relay protected-resource metadata responds. |
| Laptop environment | `127.0.0.1:3100/mcp` | HTTP GET | HTTP 405 | Route exists and expects POST. No MCP request was submitted. |
| Laptop environment | `127.0.0.1:3100/connections/start` | HTTP | HTTP 307 | Redirect behavior responds. |
| Laptop environment | `10.10.1.36:3000`, `:3100` | TCP | Connected | These are the workstation’s own interface addresses, not a test from another LAN host. |
| Laptop environment | `10.10.1.36:22` | TCP | Refused | No SSH listener on this address/port. |
| Laptop environment | `orangepi` | ICMP/DNS and SSH/TCP 22 | Failed name resolution | Therefore Orange Pi SSH reachability could not be tested. |
| Laptop environment | `sso.farismnrr.com` | HTTPS | HTTP 200; nginx header | This workstation can reach the `100.89.159.121` answer. Does not establish public Internet reachability. |
| Laptop environment | `relay.farismnrr.com` | HTTPS | HTTP 200; Cloudflare headers | Relay discovery reachable through Cloudflare. |
| Orange Pi | Laptop LAN address/ports | ICMP/TCP | Not tested | Orange Pi identity and address were unavailable. |

## 9. Feasible Tunnel Options

All options below are technical comparisons, not implementations. Orange Pi access, public routing, and router/NAT facts are insufficient to select one conclusively.

| option | laptop process | Orange Pi process | required inbound Internet ports | required outbound connectivity | TLS termination | reconnect / boot persistence | LAN IP changes / laptop offline | HTTP, OAuth, MCP, streaming | security boundary and operational complexity | DNS changes |
|---|---|---|---|---|---|---|---|---|---|---|
| SSH reverse tunnel | SSH client with reverse forwards | SSH server; reverse proxy if serving hostnames | Usually SSH port on Orange Pi; public HTTPS ports for ingress if Orange Pi serves TLS | Laptop outbound SSH to Orange Pi | At proxy on Orange Pi, or upstream TLS passthrough | SSH alone needs supervision/service management for robust reconnect and boot start | Independent of laptop LAN IP; unavailable while the laptop is asleep/offline | Generally compatible when proxy preserves HTTP semantics and supports long-lived streams/timeouts | SSH account/key and forwarding restrictions are critical; simple topology, moderate proxy/tunnel operations | Hostnames must resolve to public ingress endpoint; existing records may be repointed |
| autossh | autossh-managed reverse SSH client | SSH server; proxy | Same as SSH reverse tunnel | Laptop outbound SSH | Same as SSH reverse tunnel | Designed to restart failed SSH sessions; persistence requires service manager | Independent of LAN IP; unavailable while the laptop is asleep/offline | Same as SSH; proxy buffering/timeouts still matter | Similar SSH boundary; adds watchdog/service configuration and monitoring | Same as SSH reverse tunnel |
| FRP | `frpc` | `frps`; reverse proxy likely | FRP server port plus public HTTPS ingress ports; exact needs depend on FRP mode | Laptop outbound FRP connection | Usually Orange Pi proxy, or FRP TLS mode depending setup | Client/server need service management and reconnect policy | Independent of LAN IP; unavailable while the laptop is asleep/offline | HTTP-compatible when configured appropriately; streaming depends on proxy/FRP configuration | Adds FRP control plane and credentials/config; more components than SSH | Hostnames route to ingress; may need DNS updates |
| Cloudflare Tunnel directly from laptop | `cloudflared` tunnel client | None | No application ingress port required at home; tunnel is outbound | Laptop outbound to Cloudflare tunnel service | Cloudflare edge; tunnel carries origin traffic | Client service can reconnect; persistence needs service manager | No LAN-IP dependence; unavailable while the laptop sleeps/offlines | HTTP/OAuth/MCP can work if routes, headers and timeouts suit long-lived requests | Avoids exposing Orange Pi and home inbound ports; Cloudflare account/tunnel configuration becomes dependency | DNS records/routes must point to Cloudflare Tunnel |
| Tailscale/WireGuard plus reverse proxy | VPN client/peer | VPN peer and/or proxy, depending topology | Tailscale may traverse outbound/control paths; WireGuard generally needs reachable UDP endpoint or relay/NAT arrangement; public HTTPS ingress still needed | VPN connectivity and proxy-to-peer path | At public reverse proxy or Cloudflare edge | VPN/service-manager reconnect behavior; boot persistence must be configured | VPN addressing avoids ordinary LAN-IP changes; offline laptop still removes services | Compatible through HTTP reverse proxy; ensure streaming/timeouts and forwarded headers | Private overlay reduces origin exposure; adds VPN identity/key and proxy trust administration | Public DNS still needs a stable ingress endpoint; Orange Pi or Cloudflare records may be needed |

## 10. Risks / Constraints

- **Public SSO DNS:** `sso.farismnrr.com` publicly answers with `100.89.159.121`, in the shared CGNAT address range. Current reachability from this workstation is not evidence that ordinary Internet clients can reach it.
- **OAuth stability:** Changing the issuer hostname changes metadata, issuer identification, and token validation expectations. Changing the resource URL affects protected-resource metadata, token audience/resource checks, and MCP client discovery. Preserve the hostnames and paths or update the contract and all dependent registrations together.
- **Callbacks:** GitHub’s callback URL must continue matching its OAuth application registration. The Relay callback must match the URL stored in SSO SQLite; that stored value was not inspected.
- **TLS:** Relay appears to terminate TLS at Cloudflare; the origin-side TLS mode and origin are unknown. SSO returned nginx headers without Cloudflare headers; its TLS endpoint appears to be nginx, but its location and public reachability are unverified.
- **Host and forwarded headers:** Relay generates absolute discovery URLs from configuration, so preserve the expected public URL independently of proxy Host rewriting. The inspected code did not consume `X-Forwarded-For` or `X-Forwarded-Proto`; do not assume forwarded headers affect application URL construction or client identity. Proxy trust configuration was not observed.
- **Origin:** Relay validates the MCP `Origin` header against its configured resource origin and explicitly permits `https://chatgpt.com`. A proxy must preserve `Origin`.
- **OAuth redirect behavior:** HTTPS public SSO configuration makes SSO cookies secure. TLS termination must leave the public scheme/issuer/callback URLs consistent; no proxy-scheme inference was observed in SSO’s URL construction.
- **MCP audience:** SSO and Relay share the assertion secret, and Relay validates issuer/audience/resource. Preserve matching issuer/resource configuration across the move.
- **Long-lived requests:** The MCP endpoint is POST. Any reverse proxy/tunnel needs suitable request-body handling, buffering behavior, idle/read timeouts, and streaming support. The app’s actual streaming behavior was not exercised in this discovery.
- **Docker exposure:** The currently running fast Compose services bind published ports to `0.0.0.0`. The base/latest Compose mappings are loopback-only. Whether WSL/NAT/firewall makes the fast bindings reachable from other LAN hosts was not tested.
- **SSH:** Orange Pi SSH forwarding policy, key permissions, tunnel account privileges, firewall rules, and `GatewayPorts` are unknown. A reverse tunnel should not rely on broad `GatewayPorts` exposure; loopback-bound tunnel listeners and tightly limited forwarding reduce unintended exposure.
- **Laptop availability:** Any laptop-hosted service becomes unavailable when the laptop sleeps, powers off, loses connectivity, or the tunnel fails. Relay pending connections are in memory and are invalidated on Relay restart.
- **Reconnect:** Persistent SSH/autossh/FRP/VPN behavior and monitoring are not present as discovered facts; they would need deliberate service supervision in a later migration.
- **Orange Pi role:** The required “no `sso-auth` or `relay-agent` on Orange Pi” condition was not verified because the host could not be reached.

## 11. Suggested Target Topologies

These are technically valid candidates, conditional on Orange Pi access, public routing, and firewall facts not yet available.

1. **Orange Pi HTTPS reverse proxy with loopback-only reverse SSH tunnel:** Public DNS/HTTPS ingress to Orange Pi; reverse proxy routes `sso.farismnrr.com` and `relay.farismnrr.com` to loopback tunnel listeners; tunnel forwards to the laptop’s Docker-published services. Orange Pi runs ingress/tunnel endpoint only, not application containers.
2. **Direct Cloudflare Tunnel from laptop:** Cloudflare routes the two public hostnames through an outbound laptop tunnel directly to local SSO and Relay. Orange Pi is bypassed.
3. **Orange Pi ingress plus VPN path to laptop:** Orange Pi reverse proxy connects to the laptop over a private Tailscale/WireGuard path; public HTTPS terminates at the proxy or Cloudflare. DNS remains at the stable ingress hostname.

For each, preserve the canonical issuer, resource, callback and metadata URLs unless all affected OAuth/MCP contracts and registrations are intentionally migrated.

## 12. Missing Information

Only the following could not be discovered:

- Orange Pi hostname, OS, architecture, LAN/public IP, default route, NAT/CGNAT status, SSH reachability/state/settings, firewall, listening ports, reverse proxy, tunnel software, and installed services.
- Whether Orange Pi runs either application.
- Orange Pi-to-laptop reachability and routing symmetry.
- Whether the workstation’s physical LAN address is dynamic; WSL shows `10.10.1.36` with a `forever` address lifetime.
- Whether the public SSO `100.89.159.121` endpoint is intentionally overlay-only and what public clients see from outside this workstation’s network.
- The actual SSO SQLite-registered Relay callback URL.
- The runtime values of non-secret URL settings; actual `.env` values were intentionally not read.
- Cloudflare origin/TLS mode and the origin IP/host behind Relay’s Cloudflare proxy.
- Whether the existing `0.0.0.0` Docker publications are reachable from other LAN devices through the physical host/WSL networking.

## 13. Exact Facts For Next Agent

```text
REPOSITORY_BRANCH=refactor/full-fe-be-relay
REPOSITORY_COMMIT=db1c0333957a4f6a46f739b1861d6ad68ad7e45f

LAPTOP_HOSTNAME=thinkpad (WSL2 environment)
LAPTOP_LAN_IP=10.10.1.36

SSO_HOST_BIND=0.0.0.0
SSO_HOST_PORT=3000
SSO_CONTAINER_PORT=3000

RELAY_HOST_BIND=0.0.0.0
RELAY_HOST_PORT=3100
RELAY_CONTAINER_PORT=3100

ORANGEPI_HOSTNAME=unknown; SSH alias orangepi does not resolve
ORANGEPI_LAN_IP=unknown
ORANGEPI_PUBLIC_IP=unknown
ORANGEPI_SSH_HOST=orangepi (unresolved)
ORANGEPI_SSH_PORT=22 (SSH config alias; reachability unverified)

ORANGEPI_REVERSE_PROXY=unknown
ORANGEPI_TUNNEL_SOFTWARE=unknown

DNS_SSO=100.89.159.121 (shared CGNAT range; no AAAA answer)
DNS_RELAY=172.67.146.168,104.21.63.143; AAAA 2606:4700:3030::6815:3f8f,2606:4700:3033::ac43:92a8 (Cloudflare)

OAUTH_ISSUER=https://sso.farismnrr.com
MCP_RESOURCE=https://relay.farismnrr.com/mcp
MCP_ENDPOINT=https://relay.farismnrr.com/mcp
PROTECTED_RESOURCE_METADATA=https://relay.farismnrr.com/.well-known/oauth-protected-resource/mcp
AUTHORIZATION_SERVER_METADATA=https://sso.farismnrr.com/.well-known/oauth-authorization-server
CHATGPT_CLIENT_ID=https://chatgpt.com/oauth/client.json
CHATGPT_REDIRECT_URI=https://chatgpt.com/connector_platform_oauth_redirect

CURRENT_TLS_TERMINATION=Relay: Cloudflare edge observed; origin-side TLS unknown. SSO: nginx response observed; endpoint location unknown.
```

## 15. Orange Pi Follow-up Discovery

Orange Pi shell access was unavailable. The configured SSH alias `orangepi` did not resolve. The report's candidate overlay address `100.89.159.121` accepted a TCP connection on port 22, but non-interactive SSH as `farismnrr` was denied (`Permission denied (publickey,password)`). That address cannot be identified as the Orange Pi without an authenticated host session or tailnet status.

```text
ORANGEPI_HOSTNAME=UNKNOWN (no authenticated Orange Pi shell)
ORANGEPI_LAN_IP=UNKNOWN
ORANGEPI_PUBLIC_IP=UNKNOWN
ORANGEPI_TAILSCALE_IP=UNKNOWN; 100.89.159.121 is an unverified candidate

LAPTOP_TAILSCALE_IP=100.82.13.112 (from prior workstation observation)
TAILSCALE_LAPTOP_REACHABLE=UNKNOWN (cannot originate a test from Orange Pi)

ORANGEPI_REVERSE_PROXY=UNKNOWN (Orange Pi configuration unavailable)
ORANGEPI_CLOUDFLARED=UNKNOWN
ORANGEPI_CLOUDFLARE_TUNNEL=UNKNOWN

SSO_CURRENT_UPSTREAM=UNKNOWN; public DNS answers 100.89.159.121 and HTTPS presents nginx, but the host/upstream mapping is unverified
RELAY_CURRENT_UPSTREAM=UNKNOWN; HTTPS is served through Cloudflare, origin is not observable from this access

SSO_TLS_TERMINATION=HTTPS certificate issuer is Let's Encrypt (YE1); response identifies nginx. The machine terminating TLS is UNKNOWN.
RELAY_TLS_TERMINATION=Cloudflare edge observed; presented certificate issuer is Google Trust Services (WE1). Origin-side TLS and termination are UNKNOWN.

SSO_APP_RUNNING_ON_ORANGEPI=UNKNOWN (could not inspect Orange Pi processes/containers)
RELAY_APP_RUNNING_ON_ORANGEPI=UNKNOWN (could not inspect Orange Pi processes/containers)
```

### Overlay and ingress facts available without Orange Pi access

- The laptop-side workstation interface reported `100.82.13.112/32`; the local `tailscale` CLI was not available, so a tailnet peer listing or `tailscale status` could not be obtained.
- From the workstation, the route to `100.89.159.121` used its `100.82.13.112` overlay-looking interface, and TCP port 22 accepted a connection. This does not establish which device owns that address.
- `sso.farismnrr.com` resolved to `100.89.159.121`; HTTPS returned an nginx response and its presented certificate issuer was Let's Encrypt (YE1). No Orange Pi mapping or proxy upstream could be read.
- `relay.farismnrr.com` and `farismnrr.com` were served through Cloudflare. Their observed public certificate issuer was Google Trust Services (WE1). No origin target, Orange Pi proxy configuration, or Cloudflare Tunnel configuration could be read.
- The configured SSH alias specifies port 22. Effective `AllowTcpForwarding`, `GatewayPorts`, `PermitOpen`, `PermitListen`, SSH listener details, firewall, installed/running ingress services, cloudflared configuration, and `ss -lntup` output remain unknown because Orange Pi SSH authentication was unavailable.
- Whether `sso-auth` or `relay-agent` runs on Orange Pi could not be confirmed.

### Orange Pi to laptop connectivity tests

None of these probes could be run from Orange Pi because no authenticated shell was available. No bind address, firewall, or service setting was changed.

| probe from Orange Pi | result |
|---|---|
| `http://100.82.13.112:3000/health` | Not run; Orange Pi shell unavailable |
| `http://100.82.13.112:3000/.well-known/oauth-authorization-server` | Not run; Orange Pi shell unavailable |
| `http://100.82.13.112:3100/.well-known/relay.json` | Not run; Orange Pi shell unavailable |
| `http://100.82.13.112:3100/.well-known/oauth-protected-resource/mcp` | Not run; Orange Pi shell unavailable |

## 16. Orange Pi Ingress and Laptop Reachability

```text
ORANGEPI_HOSTNAME=orangepi3b
ORANGEPI_LAN_IP=192.168.10.5/24 (enx9c69d3b546c6); 10.10.1.1/24 (end0)
ORANGEPI_TAILSCALE_IP=100.89.159.121
ORANGEPI_PUBLIC_IP=103.130.18.204 (outbound IPv4 observed)

ORANGEPI_PROXY=nginx-proxy for sso.farismnrr.com; relay ingress is Cloudflare-fronted, origin route unconfirmed
ORANGEPI_PROXY_CONTAINER=nginx-proxy (nginx:alpine; host network; restart always)

SSO_OLD_CONTAINER=masih-awam-sso-auth-1 (running; ghcr.io/farismnrr/agentic-ai-code-sso-auth:latest)
RELAY_OLD_CONTAINER=masih-awam-relay-agent-1 (running; ghcr.io/farismnrr/agentic-ai-code-relay-agent:latest)

SSO_CURRENT_UPSTREAM=127.0.0.1:5000 -> masih-awam-sso-auth-1:5000 over Docker host-published loopback port
RELAY_CURRENT_UPSTREAM=UNKNOWN; no relay.farismnrr.com nginx vhost; active token-authenticated Cloudflared tunnel has no local hostname/origin config; relay container is reachable at 127.0.0.1:5001 -> masih-awam-relay-agent-1:3100

SSO_TLS_TERMINATION=nginx-proxy on Orange Pi; Let's Encrypt certificate at /etc/letsencrypt/live/sso.farismnrr.com/fullchain.pem; public DNS resolves directly to Orange Pi Tailscale IP, not Cloudflare anycast
RELAY_TLS_TERMINATION=Cloudflare edge observed; Google Trust Services certificate; origin/Tunnel route and Cloudflare-to-origin transport are not locally observable

CLOUDFLARED_RUNNING=yes (cloudflared.service active, enabled; running tunnel with token authentication; no token value recorded)
CLOUDFLARE_TUNNEL_PRESENT=yes; hostname routes are remotely managed/not visible in local config, so Relay Tunnel-vs-ordinary-proxy origin routing is unconfirmed

LAPTOP_TAILSCALE_IP=100.82.13.112

LAPTOP_SSO_REACHABLE_FROM_ORANGEPI=yes (both direct HTTP probes returned 200)
LAPTOP_RELAY_REACHABLE_FROM_ORANGEPI=yes (both direct HTTP probes returned 200)

SSO_LAPTOP_HEALTH_STATUS=200
SSO_LAPTOP_METADATA_STATUS=200; issuer, authorization_endpoint, token_endpoint present
RELAY_LAPTOP_DISCOVERY_STATUS=200; id, connection, connectUrl present
RELAY_LAPTOP_PRM_STATUS=200; resource and authorization_servers present

HOST_HEADER_BEHAVIOR=SSO nginx explicitly sets Host to $host; no Relay nginx vhost
ORIGIN_HEADER_BEHAVIOR=SSO nginx does not override Origin; default request-header forwarding applies; no Relay nginx vhost
FORWARDED_PROTO_BEHAVIOR=SSO nginx sets X-Forwarded-Proto to $scheme

MCP_PROXY_BUFFERING=SSO nginx does not override proxy_buffering; Nginx default is on. No Relay nginx vhost.
MCP_PROXY_READ_TIMEOUT=SSO nginx does not override proxy_read_timeout; Nginx default is 60s. No Relay nginx vhost.

DIRECT_UPSTREAM_SWAP_FEASIBLE=yes; Orange Pi reached both laptop service ports over Tailscale. SSO's existing nginx upstream can be changed while keeping its hostname/TLS endpoint; Relay requires updating its Cloudflare Tunnel origin route or adding a Relay nginx vhost because none currently exists. Public URLs and OAuth issuer/MCP resource can remain unchanged. MCP long-running/streaming responses may require revisiting buffering and the 60s read timeout.
```

Observed container details:

| container | published ports | Docker network / container IP / aliases | restart | mounts |
|---|---|---|---|---|
| `masih-awam-sso-auth-1` | `127.0.0.1:5000 -> 5000/tcp`; `3000/tcp` is not published | `masih-awam_default`, `172.22.0.2`, aliases `masih-awam-sso-auth-1`, `sso-auth` | `always` | none |
| `masih-awam-relay-agent-1` | `127.0.0.1:5001 -> 3100/tcp` | `masih-awam_default`, `172.22.0.3`, aliases `masih-awam-relay-agent-1`, `relay-agent` | `always` | none |
| `nginx-proxy` | host network; listens on `0.0.0.0:80` and `0.0.0.0:443` | host network; no bridge IP/aliases | `always` | read-only: `/home/farismnrr/Documents/Programs/nginx-proxy/nginx.conf` -> `/etc/nginx/nginx.conf`; `conf.d` -> `/etc/nginx/conf.d`; `stream.d` -> `/etc/nginx/stream.d`; `letsencrypt` -> `/etc/letsencrypt`; `certs_flat` -> `/etc/nginx/certs` |

The inspected active nginx configuration has an HTTP and HTTPS vhost for `sso.farismnrr.com`. Its HTTPS location proxies to `http://127.0.0.1:5000`, preserves `Host` with `$host`, appends `X-Forwarded-For` with `$proxy_add_x_forwarded_for`, sets `X-Forwarded-Proto` to `$scheme`, uses upstream HTTP/1.1, and sets WebSocket `Upgrade` and `Connection` headers. It does not explicitly set proxy buffering or proxy connect/read/send timeouts; Nginx defaults apply: buffering on and 60 seconds for connect, read, and send timeouts. No exact `relay.farismnrr.com` or apex `farismnrr.com` vhost was found in the active nginx server blocks.

`cloudflared.service` is active and enabled, and its process runs `cloudflared tunnel run` with token authentication. No local config argument or ingress hostname configuration was found; the only file under the checked Cloudflared config directories was `cert.pem`. The service's token value and certificate contents were not read or printed. Its running state and Cloudflare-fronted Relay response do not reveal whether Relay uses a Cloudflare Tunnel route or ordinary Cloudflare proxying to another origin.

The Orange Pi reports Armbian 25.8.1 (Debian GNU/Linux 12 Bookworm), `aarch64`, default route via `192.168.10.1` on `enx9c69d3b546c6`, and uptime of 2 days, 19 hours, 56 minutes at inspection. `tailscale ip -4` returned `100.89.159.121`; `tailscale status` listed peer `100.82.13.112` as `thinkpad`, Windows, active and direct. Thus `100.89.159.121` belongs to this Orange Pi.

### Current confirmed ingress path

```text
sso.farismnrr.com
  -> DNS A 100.89.159.121 (Orange Pi Tailscale address)
  -> nginx-proxy TLS termination (Let's Encrypt)
  -> http://127.0.0.1:5000
  -> masih-awam-sso-auth-1:5000

relay.farismnrr.com
  -> Cloudflare edge TLS termination
  -> [origin route not available in local config; cloudflared tunnel is active]

Local Relay container listener:
  127.0.0.1:5001 -> masih-awam-relay-agent-1:3100
  (whether this is the configured Cloudflare origin was not confirmed)
```

### Future path

**NOT IMPLEMENTED**

```text
Internet / existing DNS and Cloudflare hostnames
  -> Orange Pi ingress (retain current public hostnames and TLS termination)
  -> Tailscale
  -> 100.82.13.112:3000 -> laptop Docker sso-auth
  -> 100.82.13.112:3100 -> laptop Docker relay-agent
```
