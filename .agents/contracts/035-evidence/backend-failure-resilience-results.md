# Plan 035 Phase 11 — Case 7: Backend (Jaeger) failure resilience

Command sequence (real, executed against the running stack):

```
$ docker stop masih-awam-jaeger
masih-awam-jaeger

--- jaeger stopped, testing app ---
$ curl -sS -o /dev/null -w 'sidebar (unauth) -> %{http_code}\n' http://localhost:3334/api/sidebar
sidebar (unauth) -> 401

$ curl -sS -o /dev/null -w 'sidebar (auth) -> %{http_code}\n' -H "Cookie: $COOKIE" http://localhost:3334/api/sidebar
sidebar (auth) -> 200

$ time curl -sS -o /dev/null -w 'timing: %{time_total}s\n' -H "Cookie: $COOKIE" http://localhost:3334/api/sidebar
timing: 0.005683s
real  0m0.012s

$ docker start masih-awam-jaeger
masih-awam-jaeger
$ docker ps --filter name=masih-awam-jaeger --format '{{.Status}}'
Up 3 seconds
$ curl -sS -o /dev/null -w '%{http_code}\n' http://localhost:16686/
200
```

## Result

With the OTLP/Jaeger collector fully stopped, both unauthenticated (401, correctly denied) and
authenticated (200, normal data) requests to `/api/sidebar` continued to succeed with no added
latency (5.6ms server time) and no error surfaced to the client. This confirms the plan's
"telemetry backend unavailable does not break the business request" requirement
(`SimpleSpanProcessor`'s export call is not on the request-fulfillment critical path — the OTel SDK
degrades the failed export silently). Jaeger was restarted afterward and confirmed healthy
(`/` returns 200) before continuing the rest of the acceptance run — left running per the task's
"leave `masih-awam-jaeger` running" requirement.

## Verdict: PASS
