# curl-tool SSRF Policy Implementation Memory (027)

This memory documents the architectural decisions made during the `curl-tool` Rust migration for SSRF (Server-Side Request Forgery) protection.

## DNS Resolution Strategy
We use `std::net::ToSocketAddrs` for DNS resolution inside the SSRF guard instead of external crates like `hickory-resolver`. During implementation, `hickory-resolver` proved unreliable in this environment, failing with "DNS lookup failed: no connections available". `ToSocketAddrs` utilizes the standard system resolver.

## Redirect Guard (`reqwest::redirect::Policy::custom`)
The SSRF guard checks destination IP addresses not only on the initial request, but also on every HTTP redirect hop.
Because the default `reqwest` policy only enforces a hop limit, we inject a custom `reqwest::redirect::Policy::custom`. This policy extracts the host from the new redirect URL, resolves it via `std::net::ToSocketAddrs`, and aborts the request via `attempt.error` if any resolved IP belongs to a blocked subnet (private, loopback, link-local, IPv4-mapped IPv6, etc.).

## Testing Redirects
Redirect security tests use deterministic local HTTP fixtures. The tests do not expose or rely on a production CLI flag or environment variable that bypasses the initial SSRF guard.

The test architecture keeps production security behavior intact while exercising redirect validation through local fixtures/test-level setup. Redirect cases cover forbidden destinations such as private IPv4, loopback, link-local, and unsafe hostname resolution. The redirect policy is expected to reject the destination before the request is allowed to proceed.

There is no `CURL_TEST_ALLOW_INITIAL`, `CURL_TOOL_TEST_ALLOW_LOCAL`, or equivalent test-only SSRF bypass in the production binary.

## `--no-guard` Semantics
The `--no-guard` flag explicitly bypasses SSRF security checks. When present, it skips the initial IP check and uses a standard bounded `reqwest::redirect::Policy::limited(10)` instead of the custom SSRF-aware redirect policy. This is the only explicit production security bypass and must remain user-visible and intentional.

## TOCTOU (Time-Of-Check to Time-Of-Use) / DNS Rebinding
The architecture bounds the TOCTOU risk through design rather than relying on flaky timing-based CI tests.
1. **Redirect rebinding**: Each redirect hop is synchronously resolved and validated by the custom redirect policy before the request proceeds.
2. **Initial request rebinding**: A hostname is resolved and validated before `reqwest` establishes the connection. Because the implementation does not pin the resolved IP, a narrow DNS rebinding race remains possible between validation and connection establishment. This is documented as accepted residual risk rather than represented as a guaranteed mitigation.

## Threat Model / DNS Rebinding Explicit Boundary

### What IS protected:
- Initial requests to IPs that are private/loopback/link-local at time of check
- Initial requests where hostname DNS resolves to a private IP at time of check
- HTTP redirects to private IPs (both literal IP and hostname-resolving)
- HTTP redirects to loopback/link-local addresses

### Accepted residual risk (documented):
- **DNS rebinding on initial request**: If a DNS record changes between validation and the subsequent socket connection, the SSRF guard can potentially be bypassed. This is accepted residual risk because the implementation does not pin the resolved IP; fully eliminating the race would require a stronger connection-pinning design with compatibility trade-offs.
- **DNS rebinding protection: NOT GUARANTEED against an attacker with DNS control.**

### What is out of scope:
- SSRF via Gopher, file://, or other non-HTTP/HTTPS schemes (curl-tool only supports HTTP/HTTPS URLs)
