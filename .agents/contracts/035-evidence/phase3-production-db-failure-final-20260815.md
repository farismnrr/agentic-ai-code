# Plan 035 Phase 3 — final production DB-failure proof

Date: 2026-08-15. The production bundle was rebuilt with `pnpm build`. A separate
Node Nitro instance listened on `127.0.0.1:3345`; only that instance received an
unreachable loopback database URL. Loki and Jaeger remained reachable.

## Client boundary

- Real `POST /api/auth/register` reached the production bundle and failed at the
  database boundary: HTTP `500`.
- Response `Content-Type`: `application/problem+json`.
- Body shape: `type`, generic `title`, `status: 500`, `instance`, and `requestId`.
- Request ID: `cf4f4e5b-1ce7-41b0-bb9c-0c92b3616640`.
- The three input canaries were not present in the response (match count: 0).

## Stdout/consola boundary

The live production stdout excerpt was bounded to:

```text
WARN  422 Unprocessable Content
ERROR [unhandled] { type: 'Error', classification: 'unclassified' }
```

The final failure request emitted no canary, response body, database URL, source,
workspace, build, node_modules, or filesystem path data. Forbidden-data sweep:
all three canary markers 0; `.output`, `/home`, `node_modules`, workspace/Cargo/
source/build path patterns 0; stack-frame patterns 0; raw `Error.message` 0.

## Loki

Loki query: `{job="ai-code-phase3-final"} |= "cf4f4e5b-1ce7-41b0-bb9c-0c92b3616640"`.
The one matching lifecycle record contained only bounded operator fields:

```json
{"operation":"http.request.lifecycle","http.request.method":"POST","route":"/api/auth/register","http.response.status_code":500,"outcome":"server_error","request.id":"cf4f4e5b-1ce7-41b0-bb9c-0c92b3616640","trace_id":"d688a760dd0d26e26a7d40c3c8df42ff","span_id":"1c15fe037ad564bd","error.type":"Error","error.classification":"unclassified"}
```

Canary and forbidden-data sweep over the captured Loki record: 0 matches.

## Jaeger

Jaeger query for service `ai-code-phase3-final` returned the matching trace
`d688a760dd0d26e26a7d40c3c8df42ff`. Its production server span was `POST`,
`/api/auth/register`, port `3345`, status `500`, `error=true`, and
`otel.status_code=ERROR`; span ID was `1c15fe037ad564bd`. No exception message,
stack, request body, canary, credential, URL, source, workspace, or filesystem
path was present. Forbidden-data sweep: 0 matches.

## Verdict

PASS — genuine production-runtime database failure, generic client error/request
ID, bounded stdout, and correlated Loki → Jaeger operator evidence, with no
forbidden canary or internal-data disclosure.
