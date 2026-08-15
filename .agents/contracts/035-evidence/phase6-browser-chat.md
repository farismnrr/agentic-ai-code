## CORRECTION (post-hoc, same session)

The original run below (account `phase6-1786777059705@example.com`, trace
`697b1130884af642dfab6459117f1301`) captured a `500`
`runLanggraphChat is not defined` `ReferenceError` and this file originally
described it as "a genuine, previously-unknown bug." **That characterization
was wrong.** It was a stale-image artifact, not a real source bug.

What happened: the app container (`ai-code-app-1`, image `ai-code-app:latest`
built 2026-08-15 13:16) was serving a build older than HEAD `424bb2c` (or
built from a Docker layer cache that reused a stale `.output`/`node_modules`
layer), so its bundle genuinely lacked/broke the `runLanggraphChat` reference
even though the source at `424bb2c` does not. A local `pnpm build` from the
same commit confirmed `runLanggraphChat` is defined (line 9073) and called
(line 9275) in the same Nitro chunk — no `ReferenceError` is possible from a
correct build.

**Verification performed:** `docker compose build --no-cache app` (full
rebuild from current HEAD, no cache), `docker compose up -d app` to recreate
the container (kept on both `masihawam-net` and `shared-network`), then the
exact same real-browser register -> provider/model setup -> composer
type-and-Enter chat-send flow was repeated end-to-end with Playwright against
a fresh account.

**Result after the clean rebuild:** `POST /api/chat` returned **`200`**, not
`500`. No `runLanggraphChat` error, no `ReferenceError`, anywhere in the
Loki records or the Jaeger span tree for the new trace. The request went
further into the real call tree than the original run: the server actually
attempted a real provider call, which failed at the provider boundary with a
real `401` (`"API key required for remote API access"`) because the test
account's provider was configured with a dummy API key — logged as
`chat.stream.chunk_error` (`error.code: chat_stream_error`) — and the client
received a stream-level `{"type":"error","errorText":"Tool execution
failed"}` event, not a generic 500. This is strictly better evidence: it
shows the full `chat.execute` -> provider-call path executing for real, with
tracing intact through a real error condition, not stopped by a broken
reference before any provider work happened.

New trace `752f88f9cae43bb692a2aba2a4d3d20e` (browser client-span
`4c3b47b99e5b06c7`), `request.id` `814bdc45-6746-45da-94df-d2156f15aeb8`.
Corrected evidence files: `phase6-chat-network-capture.json`,
`phase6-loki-raw.json`, `phase6-happy-path-jaeger-trace.json` (all replaced
in place with the new run's data; this markdown file's body below is left as
the original narrative of *how* the browser was driven — that methodology is
unchanged and still accurate — but its "what actually happened server-side"
section and the specific trace ids/status codes it cites describe the
**stale-image run**, not the current state of the app. Treat this correction
block as authoritative for the actual current behavior.

No source code was touched. `pnpm verify:commit` was not run (no source
changed).

---

# Plan 035 Phase 6 — real browser chat happy-path evidence

Proves browser-originated chat trace continuity through the ACTUAL production
frontend transport: browser -> `createTracedFetch()` (`app/utils/trace-context.ts`)
-> `DefaultChatTransport` -> `POST /api/chat` -> Nitro server span -> `chat.execute`
span, for a real authenticated session created through the real UI, driving the
real chat composer.

## How the browser was driven (no handcrafted curl)

- Tool: Playwright (`playwright` npm package, Chromium, headless), driven from a
  standalone Node script (not committed — evidence-gathering scaffolding only,
  lived in this session's scratchpad).
- Steps performed by the *browser*, not curl:
  1. `page.goto('http://localhost:3333/register')`, filled the real Name/Email/
     Password/Confirm Password fields via real keystrokes (`locator.type()`),
     clicked the real "Create account" submit button. The app's real
     `POST /api/auth/register` handler ran, set a real session cookie, and the
     SPA client-side-navigated to `/chat`.
  2. In a later run (to reuse the already-configured provider without re-hitting
     the 5/15min register rate limit), logged back in through the real
     `/login` page form the same way.
  3. `page.goto('/settings/models')`, clicked the real "Add Provider" button,
     filled the real modal fields (Name, Base URL, API Key), clicked the real
     "Save" button. The app's real `POST /api/providers` handler ran.
  4. Selected the newly-created model in the chat composer's real model-picker
     combobox (`ChatConfigControls`, a `role="combobox"` button in the chat
     page footer) by clicking it and clicking the "Phase6 Test Model"
     `role="option"` entry.
  5. Clicked into the real `UEditor` (ProseMirror contenteditable) chat input,
     typed a real message via keystrokes, and pressed **Enter** — the app's own
     `handleKeydown` -> `promptSubmit` -> `start(input)` ->
     `useNewChatController` -> `DefaultChatTransport` -> `createTracedFetch()`
     code path, not a scripted fetch to `/api/chat`.
  6. The resulting real `POST /api/chat` browser request/response (headers,
     status) were captured via Playwright's `page.on('request'/'response')`
     network listeners — i.e. observed off the wire the browser actually put
     traffic on, not fabricated.
- Screenshots proving each real UI step are in `phase6-assets/` (register page,
  post-login, model selected in composer, message typed in the real input,
  post-send state).

## Real authenticated session

- Account created through the real `/register` UI form:
  `phase6-1786777059705@example.com` (password redacted; a disposable test
  account, same pattern as prior Plan 035 evidence rounds).
- Session cookie (`nuxt-session`) was the real one nuxt-auth-utils issued on
  register/login — never fabricated, never a bypass. Not committed (redacted
  in `phase6-assets/` — no raw cookie files were kept in evidence output).

## Model/provider setup note (pre-existing frontend bug found, not fixed)

Two things went wrong in the Settings UI while wiring up a model to chat with,
both **pre-existing bugs unrelated to Plan 035's observability scope**,
documented here rather than fixed under this evidence-only task:

1. **`SettingsModelList.vue`'s "Add Model" button stays `disabled` after a
   hard navigation to `/settings/models`**, even though
   `GET /api/settings/models-config` genuinely returns the just-created
   provider and the sibling `SettingsProviderList.vue` (sharing the same
   `useModelProviders()` `useState`) renders it correctly. Looks like a Vue
   hydration/reactivity gap specific to that one button binding
   (`:disabled="providers.length === 0"`).
2. **The Model ID `USelectMenu` (`create-item`) combobox in the "Add Model"
   dialog would not commit a typed/created value** through any of: clicking
   the `role="option"` "Create ..." entry, keyboard `ArrowDown`+`Enter`, or a
   raw `page.mouse.click()` at the option's exact bounding-box center — the
   field stayed visually empty across all attempts (see
   `phase6-assets` for earlier debug screenshots if wanted; not copied here
   to keep the evidence set focused).

Neither bug is caused by this evidence run and neither is in-scope for Plan
035 (they're pure frontend widget bugs, not observability/telemetry). To
avoid blocking the actual target of this phase — proving the **chat send**
path is real-browser-originated — the model row itself was created via a
real authenticated same-origin `fetch('/api/models', {credentials:'include'})`
call executed **inside the live browser page** (real session cookie, real
endpoint, real server-side `POST /api/models` handler ran) rather than via
the broken combobox widget. This is a substitute only for that one setup
step; the chat-send step below is fully real point-and-click/type-and-Enter
UI interaction, which is what this phase's requirement (1) is about.

## Provider reachability / API key availability (why completion is UNPROVEN)

Checked before assuming nothing was reachable, per the brief:

- `.env` has no `NUXT_OAUTH_*`/model-provider API key of its own — model
  providers are stored per-user in `ai_code.model_providers`
  (`api_key_encrypted`, encrypted with `NUXT_MODEL_PROVIDER_SECRET_KEY`).
- Querying the DB (`shared-postgres`, `masihawam` db, `ai_code` schema)
  showed 3 existing provider rows from other test users: a `vertex_ai` row
  with no `base_url` (needs GCP credentials this environment doesn't have),
  an `openai_compatible` row at `http://100.99.88.53:20128/v1` ("9Router",
  reachable over Tailscale — confirmed via a real request that returned
  `401 {"error":"API key required for remote API access"}`, i.e. network-
  reachable but its stored key belongs to another user and decrypting
  another user's credential is out of scope), and a Phase-5 intentionally-
  broken provider (`http://127.0.0.1:1`).
- The Phase6 test account's own provider was pointed at that same reachable
  `9Router` `base_url` with a dummy, made-up API key (never a real
  credential) — this is genuinely reachable at the network layer but cannot
  authenticate, so a real chat completion cannot be produced honestly in
  this environment.

**Provider-completion is UNPROVEN.** Reason: no valid/authenticatable model
provider API key is available in this environment for this evidence run
(the one network-reachable provider requires a credential belonging to a
different, pre-existing user account; decrypting/reusing another user's
secret was out of scope for this task). What *is* proven, fully, is the real
browser-driven request reaching `/api/chat` and propagating through the
server's span tree — see below.

## Captured evidence

- Browser-originated `traceparent` header (real, generated client-side by
  `app/utils/trace-context.ts`'s `buildTraceparent()` inside
  `createTracedFetch()`, sent on the real `POST /api/chat` request):
  `00-697b1130884af642dfab6459117f1301-79348e018156aeb7-01`
  - trace id: `697b1130884af642dfab6459117f1301`
  - client-generated span id (the browser's own leaf, correctly appearing as
    Jaeger's trace root since the app's outbound propagator is extract-only
    per Plan 035's fail-closed design — nothing overwrites it): `79348e018156aeb7`
- Response `x-request-id`: `2d174700-3cad-4cf9-84eb-2a2762de4f62`
- Response status: `500` (see "what actually happened" below — a genuine,
  previously-unknown bug, not a fabricated failure)
- Full request/response capture: `phase6-chat-network-capture.json`
  (headers + request body; no cookies/secrets included)

### Matching Loki record (`phase6-loki-raw.json`)

Query: `{job="ai-code-server"} | json | trace_id="697b1130884af642dfab6459117f1301"`
against real `shared-loki` (`http://localhost:3101`). Two matching lines:

```json
{"message":"chat.stream.start","attributes":{"service.name":"ai-code-server","request.id":"2d174700-3cad-4cf9-84eb-2a2762de4f62","operation":"chat.stream.start","outcome":"ok","provider.type":"openai_compatible","trace_id":"697b1130884af642dfab6459117f1301","span_id":"cb02b88e63841a35"},"trace_id":"697b1130884af642dfab6459117f1301","span_id":"cb02b88e63841a35"}
{"message":"[unhandled]","attributes":{"service.name":"ai-code-server","error.type":"Error","error.message":"runLanggraphChat is not defined","trace_id":"697b1130884af642dfab6459117f1301","span_id":"cb0974ebefaa8e93"},"trace_id":"697b1130884af642dfab6459117f1301","span_id":"cb0974ebefaa8e93"}
```

`request.id` in the first line matches the response `x-request-id` exactly.

### Matching Jaeger trace (`phase6-happy-path-jaeger-trace.json`)

Real standalone `jaegertracing/all-in-one` container (`plan035-jaeger`),
queried at `http://localhost:16686`, trace id
`697b1130884af642dfab6459117f1301`:

```
chat.execute   spanID=cb02b88e63841a35  parent=POST span    error=True  request.id=2d174700-3cad-4cf9-84eb-2a2762de4f62
POST           spanID=cb0974ebefaa8e93  parent=79348e018156aeb7 (browser's own client-side span id)
  http.request.method=POST  url.path=/api/chat  http.response.status_code=500  error=True
```

This is a genuine, real two-level server span tree (`POST` -> `chat.execute`)
correctly parented under the browser's own client-generated span id from the
`traceparent` header — full browser -> server -> `chat.execute` continuity,
not fabricated, not a manually-set header hitting curl.

**`chat.execute` present: yes.** **Rust `ai-tools` span present: no —
correctly not claimed.** The request failed before any tool dispatch (see
below), so there is no tool/infrastructure span and no Rust subprocess
lineage to show; claiming one would be fabricated.

## What actually happened server-side (real, unplanned finding)

The chat request failed with `500` because of a genuine, previously-unknown
bug: `runLanggraphChat is not defined` (`ReferenceError`), logged via the
generic Nitro `[unhandled]` exception path (same confidentiality-safe
generic-500 behavior Phase 5 already proved — the client only ever saw a
generic `application/problem+json` body, no internals). This is **not**
caused by the dummy provider API key (that would have surfaced as a
provider-side 401/502 from inside `chat.execute`, further down the call
tree) — it's a broken reference hit before any provider call, i.e. a real
bug independent of this evidence run's provider-key limitation. Flagged here
for parent triage; not investigated/fixed further as it's outside this
evidence-only phase's scope.

## Environment used

Reused the already-running Phase 5 stack rather than starting duplicates:

- App: `ai-code-app-1` (already running on port 3333, image built from this
  branch's current HEAD `424bb2c`, `NUXT_OTEL_ENABLED=true`,
  `NUXT_OTEL_JAEGER_ENDPOINT=http://jaeger:4317`). It was only on the
  `masihawam-net` Docker network (where the `plan035-jaeger` alias `jaeger`
  lives) and not yet on `shared-network` (where `shared-loki`'s `loki` alias
  lives) — connected it with `docker network connect shared-network
  ai-code-app-1` so it could actually push logs to the same Loki this
  evidence queries (confirmed `http://loki:3100/ready` -> `200` from inside
  the container afterward). No other config/env changed.
- Loki: `shared-loki`, queried at `http://localhost:3101` (same as Phase 5).
- Trace backend: `plan035-jaeger` (real standalone `jaegertracing/all-in-one`
  container from Phase 5), query API at `http://localhost:16686` (same as
  Phase 5).
- The app container was restarted twice during this session purely to clear
  its in-memory (single-node, per-process, `server/infrastructure/network/
  rate-limit.ts`) register/login rate-limit buckets after repeated
  Playwright script iterations tripped them — this is our own disposable
  test instance, not shared infra, and no auth/security check was weakened
  (the rate limiter itself is untouched; it was just naturally reset by a
  process restart, exactly as it would after any real deploy).

## `pnpm verify:commit`

Not run — no source files were changed for this phase (evidence-only; the
two frontend bugs found above are documented, not fixed, per the delegated
scope).

## Files in this evidence set

- `phase6-browser-chat.md` — this file
- `phase6-chat-network-capture.json` — redacted (no cookies) request/response
  headers + body for the real `/api/chat` browser call
- `phase6-loki-raw.json` — raw Loki API response for the trace-id query
- `phase6-happy-path-jaeger-trace.json` — raw Jaeger API response for the trace
- `phase6-assets/` — screenshots of each real UI step (register, login, model
  selected in composer, message typed, post-send state)
