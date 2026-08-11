# 027 - SearXNG Deterministic Fixtures

## Context
During the Rust CLI refactor for the `searxng-search-tool`, we needed to ensure that our CI does not rely on the public internet (or a live SearXNG instance) for integration tests. Relying on external services for tests often leads to flaky CI builds due to rate limits, network interruptions, or changing API responses.

## Solution
We implemented a deterministic mock HTTP server within the test suite (`packages/rust-tools/tests/searxng_tool_tests.rs`) using a lightweight local `std::net::TcpListener` running in a separate thread on a randomly assigned port.

This allows us to accurately test and simulate various edge cases that would be hard to trigger against a real server:

1. **Successful Responses**: Tests correct parsing and formatting of expected JSON structures.
2. **Empty Results**: Tests graceful handling of successful queries that yield no matches.
3. **Malformed JSON**: Ensures the tool handles invalid API payloads without crashing, returning a proper error string instead.
4. **Timeouts**: The mock server can intentionally pause its thread (`thread::sleep`) longer than the configured client timeout (5 seconds), proving that the tool correctly aborts and returns an error message instead of hanging indefinitely.
5. **Connection Failures**: Simulates total network unavailability by pointing the tool to a known closed port, validating the exact error surfaced to the CLI user.
6. **HTTP 4xx/5xx**: Validates that HTTP-level errors are intercepted and reported accurately, rather than being parsed as search results.

By replacing external network calls with this mock server, we guarantee that the integration tests run fast, reliably, and deterministically on any CI runner.
