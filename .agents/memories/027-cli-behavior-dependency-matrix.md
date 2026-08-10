# Rust CLI Behavior & Dependency Matrix

As part of the JS-to-Rust CLI migration (Plan 027), this matrix documents the functional baseline, intended boundaries, and primary dependencies for the three migrated CLI tools. This ensures future maintenance does not inadvertently bleed into the Nuxt application layer or violate security invariants.

## 1. Architectural Boundary

| Area | Language / Stack | Responsibility |
|---|---|---|
| **Executable CLI Layer** | **Rust** (`packages/rust-tools`) | Terminal parsing, subprocess lifecycle, HTTP proxying, execution guards (SSRF, timeout). |
| **Application Runtime** | **TypeScript / Nuxt** | User interface, state management, API routes, tool factory definitions. |
| **Tool Factories** | **TypeScript** | Defining tool schemas for LLMs and delegating execution to the Rust binaries. |

*Invariant:* The Rust binaries act as headless executors. They do not know about LLMs, Vue, or the web application.

## 2. CLI Behavior Matrix

| Tool | Core Behavior | Security / Constraints | Output Format |
|---|---|---|---|
| `terminal-tool` | Executes arbitrary executable commands via POSIX-compliant argument parsing (avoids shell interpolation). | Requires `--no-guard`. Strict timeout (default 30s) must deterministically reap the child process. Prevents uncontrolled descendant leaks. | `Exit: <code>\nStdout: <out>\nStderr: <err>` |
| `curl-tool` | Executes HTTP requests (GET, POST, etc.) and returns responses. | **SSRF Guard:** Blocks loopback, private networks, link-local, and multicast (IPv4/IPv6). Redirects must be bounded or dropped. Bypassable only with `--no-guard`. | Status code, headers, and body. |
| `searxng-search-tool` | Proxies search queries to a SearXNG instance. | Expects valid JSON responses. Must gracefully handle network timeouts, empty results, 5xx errors, and malformed JSON. | JSON string matching the SearXNG schema or structured error. |

## 3. Dependency Matrix (Rust Tools)

| Tool | Key Dependencies | Purpose |
|---|---|---|
| **All Tools** | `clap` | Strict, deterministic CLI argument parsing. |
| **All Tools** | `tokio` | Async runtime for HTTP requests and subprocess timeouts. |
| `terminal-tool` | `shell-words` | Safely tokenizing raw command strings without invoking `/bin/sh`. |
| `curl-tool` | `reqwest` | HTTP client. Must handle redirects safely (or disable them). |
| `curl-tool` | `std::net::ToSocketAddrs` | Synchronous DNS resolution to prevent DNS rebinding attacks and validate IPs before connection. |
| `searxng-search-tool`| `reqwest`, `serde_json`| HTTP requests to the search engine and JSON payload parsing. |

## 4. Maintenance Notes

- **Modifying Tools:** Any addition to the CLI arguments must be tested natively via `cargo test` and updated in the corresponding TypeScript factory.
- **Security Policy:** SSRF rules in `curl-tool` must evolve independently of application-level URL parsing. If the Nuxt application needs to fetch a private URL, it should bypass the CLI or pass `--no-guard` explicitly.
- **No JS Fallback:** The old `.mjs` CLI wrappers are permanently deleted. Do not attempt to use `node` to execute these binaries.
