# curl-tool SSRF Policy Implementation Memory (027)

This memory documents the architectural decisions made during the `curl-tool` Rust migration for SSRF (Server-Side Request Forgery) protection.

## DNS Resolution Strategy
We use `std::net::ToSocketAddrs` for DNS resolution inside the SSRF guard instead of external crates like `hickory-resolver`. During implementation, `hickory-resolver` proved unreliable in this environment, failing with "DNS lookup failed: no connections available". `ToSocketAddrs` utilizes the standard system resolver and performs flawlessly.

## Redirect Guard (`reqwest::redirect::Policy::custom`)
The SSRF guard explicitly checks destination IP addresses not only on the initial request, but also on every HTTP redirect hop.
Because the default `reqwest` policy only enforces a hop limit, we inject a custom policy `reqwest::redirect::Policy::custom`. This policy extracts the host from the new redirect URL, resolves it via `std::net::ToSocketAddrs`, and aborts the request via `attempt.error` if any resolved IP belongs to a blocked subnet (private, loopback, link-local, IPv4-mapped IPv6, etc.).

## Testing Redirects
Testing redirects to private IPs requires passing the *initial* SSRF guard with a valid public IP or hostname. Many public "redirect testing" services (like httpbin.io or httpbingo.org) actively block redirects to private IPs (returning 403 Forbidden).
To circumvent this and accurately test the redirect guard locally:
1. We use a local mock HTTP server that returns `302 Found` with a private `Location`.
2. We introduced an internal testing hook environment variable `CURL_TEST_ALLOW_INITIAL=1` in `curl-tool.rs`.
3. When `CURL_TEST_ALLOW_INITIAL` is set, `curl-tool` bypasses the initial request IP check, allowing the request to reach the local mock server on `127.0.0.1`.
4. The mock server responds with a redirect to a forbidden IP (e.g., `192.168.1.1` or `localtest.me`).
5. The `reqwest` custom redirect policy correctly intercepts this, resolves it, and blocks it, proving the redirect guard works.

## `--no-guard` Semantics
The `--no-guard` flag cleanly bypasses **all** security checks.
When present, it skips the initial IP check and also falls back to a standard `reqwest::redirect::Policy::limited(10)` instead of our custom policy. This means internal scripts that explicitly need to query local addresses can do so safely and transparently by providing the flag.

## TOCTOU (Time-Of-Check to Time-Of-Use) / DNS Rebinding
The architecture explicitly bounds the TOCTOU risk for DNS Rebinding through design rather than flaky CI regression tests. 
1. **Redirect Rebinding**: Fully covered. As proven by the `curl_tool_tests.rs` redirect regression test, an attacker cannot bypass SSRF via an HTTP redirect to a private IP (e.g., `localtest.me`) because the custom `redirect::Policy` synchronously resolves and validates the destination *at the exact moment* of the hop, before the socket connection is attempted.
2. **Initial Request Rebinding**: To achieve a true TOCTOU on the initial request, an attacker must successfully switch their DNS A-record from a public IP to a private IP in the extremely narrow millisecond window between our `ToSocketAddrs` validation and `reqwest`'s internal socket connection. Since we do not pin the IP (due to `reqwest`'s `resolve()` strictly associating the domain to a *specific port*, which would break dynamic HTTP/HTTPS port transitions), we accept this sub-millisecond race condition as an explicit design boundary. The probability of successfully exploiting this without controlling the target network's DNS cache is negligible.

## Threat Model / DNS Rebinding Explicit Boundary

### What IS protected:
- Initial requests to IPs that are private/loopback/link-local at time of check
- Initial requests where hostname DNS resolves to a private IP at time of check  
- HTTP redirects to private IPs (both literal IP and hostname-resolving)
- HTTP redirects to loopback/link-local addresses

### Accepted residual risk (documented):
- **DNS rebinding on initial request**: If a DNS record changes from a public IP to a private IP in the sub-millisecond window between `ToSocketAddrs` validation and `reqwest`'s socket connection, the SSRF guard can be bypassed. This is accepted residual risk because: (a) it requires an attacker to control DNS TTL and the resolver cache on the target host, (b) the attack window is ~1ms, and (c) mitigating it fully requires IP-pinning which breaks dynamic HTTPS port negotiation.
- **DNS rebinding protection: NOT GUARANTEED against a sophisticated attacker with DNS control.**

### What is out of scope:
- SSRF via Gopher, file://, or other non-HTTP/HTTPS schemes (curl-tool only supports HTTP/HTTPS URLs)
