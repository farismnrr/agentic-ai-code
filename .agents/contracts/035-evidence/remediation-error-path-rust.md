# Plan 035 Lane 6 — Remediation re-proof: Error path C (Rust relay internal/protocol failure)

## Setup

`target/release/ai-tools relay --mode local --dir /tmp/relay-test-dir --execution-root
/tmp/relay-test-dir --origin http://localhost:3334 --port 47821` run on the host with
`NUXT_OTEL_ENABLED=true NUXT_OTEL_JAEGER_ENDPOINT=http://localhost:4317`.

## Trigger

A real `tools/call` request against `POST /mcp` deliberately missing the required
`mcp-name` standard header (a malformed/incomplete tool call from the client's perspective) —
attempting to progressively satisfy the relay's strict header/body cross-validation surfaced this
as the reachable failure once `mcp-protocol-version`, `_meta` protocol-version/clientCapabilities,
and `mcp-method` were supplied but `mcp-name` was still missing.

## Client-visible response

```
HTTP/1.1 400 Bad Request
x-request-id: afcd980a-4725-4e02-b813-1c2199b22338
content-type: application/json

{"jsonrpc":"2.0","id":3,"error":{"code":-32020,"message":"Header mismatch: required standard header 'mcp-name' is missing for method 'tools/call'"}}
```

Generic, protocol-level message only — no stack trace, no filesystem paths, no internal relay
state leaked.

## Correlated Jaeger trace (`service=ai-code-relay`)

`GET http://localhost:16686/api/traces?service=ai-code-relay` -> trace `a521cb7c63331a19edf170a89cb044d0`:

```json
{
  "operationName": "relay.request",
  "tags": [
    { "key": "request_id", "value": "afcd980a-4725-4e02-b813-1c2199b22338" },
    { "key": "otel.scope.name", "value": "ai-code-relay" },
    { "key": "code.file.path", "value": "packages/rust-tools/infrastructure/src/transport.rs" }
  ],
  "logs": [
    {
      "fields": [
        { "key": "event", "value": "relay.mcp.header_validation" },
        { "key": "level", "value": "WARN" },
        { "key": "outcome", "value": "rejected" }
      ]
    }
  ]
}
```

`request_id` on the span matches the client `x-request-id` header exactly — the real diagnostic
(`relay.mcp.header_validation` / `outcome: rejected`) is available to the operator via this trace,
while the client only sees the generic protocol-level message. Full raw trace response saved as
`remediation-error-path-rust-jaeger-trace.json`; raw response body saved as
`remediation-error-path-rust-response.txt`.

## Verdict: PASS

Real Rust-side failure produced a generic client-visible error and a correlated, request-id-tagged
diagnostic span visible only to the operator via Jaeger — same public/private split pattern as the
Nuxt-side error paths, now confirmed for the Rust relay under live OTLP export.
