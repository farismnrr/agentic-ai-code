# Connect external MCP client to AI Code's MCP relay

external MCP client connects to the same public OAuth-protected MCP resource used by other remote clients. There is no external MCP client-only transport inside this repository.

Before starting, complete:

1. [Keycloak / Authorization Server setup](keycloak.md)
2. [Remote MCP deployment](remote-mcp.md)
3. unauthenticated public smoke checks
4. preferably an authenticated owner-token `server/discover` + `tools/list` smoke check

## 1. Use the public MCP URL

Your connection URL must be the canonical HTTPS resource, for example:

```text
https://mcp.example.com/mcp
```

Do not use the relay's `127.0.0.1:47821` address in external MCP client.

## 2. Enable external MCP client developer mode

OpenAI's current external MCP client documentation calls custom remote MCP integrations **apps** and exposes them through developer mode. The exact availability and write-action permissions depend on the external MCP client plan/workspace. Full MCP write/modify support is currently documented for Business and Enterprise/Edu workspaces; product availability can change independently of this repository.

Current UI paths documented by OpenAI include:

- Business admin/owner: **Workspace settings -> Apps -> Create**;
- Enterprise/Edu authorized user: enable developer mode under **Settings -> Apps -> Advanced Settings**, then create from **Apps -> Create**;
- workspace admins can also control developer-mode access through workspace permissions/RBAC.

Official reference: [Developer mode and MCP apps in external MCP client](https://help.openai.com/en/articles/12584461-developer-mode-and-full-mcp-connectors-in-external-mcp).

## 3. Create the app and scan tools

In external MCP client:

1. choose **Apps -> Create**;
2. enter the public MCP endpoint and required app metadata;
3. choose OAuth authentication;
4. choose **Scan Tools**;
5. complete the authorization prompt against Keycloak;
6. wait for tool discovery to finish;
7. create/save the app.

When external MCP client discovers the protected resource, it should follow the relay's Bearer challenge and Protected Resource Metadata to the configured Authorization Server.

The product UI can change over time, so prefer OpenAI's current help page over old screenshots. The server-side contract remains standard MCP + OAuth.

## 4. Configure the callback exactly

During OAuth setup, external MCP client will provide/use a callback URI. Configure the Authorization Server to allow the **exact callback URI shown by the current connection flow**.

Do not reuse a callback URI copied from another account/session/environment and do not broadly allow arbitrary redirects.

For dynamic registration, apply the restrictions described in [keycloak.md](keycloak.md).

## 5. Complete owner login

Authenticate as the same external identity whose token `sub` is configured as:

```text
OAUTH_OWNER_SUBJECT
```

The resulting access token must include:

- audience/resource: exact MCP URL;
- scope: `relay.coding`;
- valid signature/time claims.

The relay will fail closed if any of these do not match.

## 6. Verify tool discovery

The immutable v15 source catalog exposes 52 retained tools. Primary advertises the 15 core terminal/workspace tools; Full advertises all 52. Local Git and LSP wrappers are intentionally absent from this public surface, so terminal remains their fallback. Refresh/recreate a client connection after upgrading the relay so `tools/list` is rediscovered.

```text
terminal_exec
ssh_readonly_exec
http_fetch
web_search
directory_list
file_search
file_write
file_edit
file_read
text_search
git_remote_list
git_remote_branch_get
git_fetch
git_push
workspace_add
workspace_list
workspace_get
workspace_remove
change_request_list
change_request_get
change_request_create
change_request_update
change_request_checks
change_request_merge
apply_patch
telegram_send_message
terminal_job_start
terminal_job_get
terminal_job_cancel
issue_list
issue_get
issue_create
issue_update
issue_comment
issue_close
issue_reopen
workflow_list
workflow_get
workflow_run_list
workflow_run_get
workflow_run_jobs
workflow_job_log_preview
dependabot_alert_list
dependabot_alert_get
code_scanning_alert_list
code_scanning_alert_get
secret_scanning_alert_list
secret_scanning_alert_get
secret_scanning_alert_locations
workflow_dispatch
workflow_run_rerun
workflow_run_cancel

```

The retained structured contracts include:

```text
directory_list, file_search, text_search, file_read, file_edit, file_write, apply_patch
git_remote_list, git_remote_branch_get, git_fetch, git_push
change_request_*, issue_*, workflow_*
ssh_readonly_exec, http_fetch, web_search, telegram_send_message
workspace_add, workspace_list, workspace_get, workspace_remove
terminal_exec, terminal_job_start, terminal_job_get, terminal_job_cancel
```

Server hard limits remain authoritative even when a caller supplies its own limit. `directory_list` caps depth at 4 and returned entries at 100; `file_search` and `text_search` cap returned matches at 100; `file_read` caps a request at 1,000 lines and 256 KiB; `file_edit` and `file_write` cap file/payload content at 1 MiB. Mutation defaults are deliberately conservative: an ambiguous `file_edit` fails, and `file_write` never replaces an existing file unless `overwrite=true`.

Prefer an active dedicated MCP capability when it fully covers the operation: workspace tools for structured file work, remote Git transport for fetch/push, forge/issues/workflows for hosted changes, SSH diagnostics for remote inspection, and HTTP/web/messaging tools for their supported requests. Keep `terminal_exec` for builds, tests, package managers, interpreters, project scripts, shell pipelines, local Git, LSP-adjacent commands, and unsupported CLIs. Terminal arguments use direct argv semantics and the same credential, privilege, SSH, and socket boundaries as the synchronous path; `terminal_job_*` provides explicit polling/cancellation for clients without MCP Tasks.

The same MCP endpoint can also advertise the bounded read-only `workspace://<repo-name>/{manifest,agent-guidance,status,head}` resources. Resource availability does not grant arbitrary file browsing.

If external MCP client shows an older catalog after the server has been upgraded, refresh/recreate the connection so the client action snapshot is rediscovered.

## 7. Test in increasing risk order

Start with a non-destructive call such as reading a version or listing a safe directory inside the execution root.

Then verify a bounded command, for example conceptually:

```text
pwd
```

Only after discovery and safe execution work should you approve a command that mutates the coding workspace.

Client-side confirmation UI is useful UX, but it is not the security boundary. The relay still enforces OAuth, filesystem scope, non-root execution, and Bubblewrap server-side.

## OAuth refresh tokens

For a long-lived external MCP client app, configure the Authorization Server to support refresh-token renewal. OpenAI's current guidance recommends OIDC providers advertise/support `offline_access` (or the provider-equivalent capability) when refresh tokens are required. Without refresh access, the app may require a new interactive login after the original access token expires.

This does not change the relay's authorization requirement: tool tokens must still include `relay.coding`. Treat `offline_access` as Authorization Server/client-session behavior, not a relay tool permission.

## Long-running and slow MCP operations

The relay no longer has an unconditional five-minute terminal ceiling. It also avoids requiring one HTTP request to remain open for work whose latency is legitimately unpredictable.

- `timeout_ms: 0` means no terminal command deadline unless the operator configured `RELAY_MAX_TERMINAL_TIMEOUT_MS`.
- `terminal_exec`, `web_search`, and read-like `http_fetch` methods (`GET`, `HEAD`, `OPTIONS`) can use optional MCP Tasks. Mutating HTTP methods remain synchronous until a later remote-mutation layer provides request-level idempotency/deduplication. A Tasks-capable client may receive a task handle and retrieve the final result through `tasks/get`; bounded native reads remain synchronous.
- The first-party Nuxt MCP client applies a separate per-HTTP-round-trip deadline (`NUXT_REMOTE_MCP_REQUEST_TIMEOUT_MS`, default 45 seconds). That deadline is not the durable task lifetime.
- Task polling honors the relay's `pollIntervalMs` hint and uses bounded backoff rather than a hot fixed polling loop.
- A dropped/timed-out HTTP round trip is not treated as implicit task cancellation. Explicit task cancellation still targets the authoritative relay job and process tree.
- Clients that do not negotiate Tasks can still use `terminal_job_start`, poll with `terminal_job_get`, and stop terminal work with `terminal_job_cancel`.
- Task input handoff is not currently used by these relay tools. If a future task reports `input_required`, the current first-party client fails explicitly rather than waiting indefinitely until a reviewed input contract exists.

external MCP client controls how progress/tool cards are rendered. The relay can provide protocol task state and results, but it cannot force external MCP client to render raw terminal output like a native terminal UI.

## What a successful connection proves

A successful OAuth connection + tool discovery + safe tool call proves the live client can reach and use the relay.

It does not automatically prove every negative case, hosted-Nuxt token ownership path, or detailed callback/token-claim internals. Keep those claims separate when troubleshooting or documenting acceptance.

## MCP tool profiles (Plan 045)

The relay supports `RELAY_TOOL_PROFILE=full|primary` (or `--tool-profile`). `full` is the default and canonical superset; `primary` is a external MCP client routing/UX subset and does not delete capabilities. Future tools are full-only until explicitly reviewed for promotion.

Primary exposes 15 core tools for terminal plus structured workspace work. Full exposes the immutable v15 retained catalog of 52 tools, including remote Git transport, HTTP/web, Full-only SSH diagnostics, forge/issues/workflows, alerts, and Telegram. Local Git and LSP wrappers are not in the public catalog; terminal remains the fallback for builds, tests, package managers, interpreters, scripts, pipelines, and uncovered CLI operations. `ssh_readonly_exec` is a normal discoverable Full tool and does not require client-side SSH parsing; clients provide only structured alias/command/args while the relay owns SSH config/key resolution and read-only enforcement. The v15 snapshot is immutable; do not rewrite v13 or v14 snapshots.

A simultaneous public Full + Primary deployment is a separate operator decision because separate endpoints may require reviewed OAuth/resource configuration. Where external MCP client Action Control can hide actions client-side, that can be used for A/B testing without a second endpoint.
