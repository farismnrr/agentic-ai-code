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

The current Plan-040 source catalog exposes 50 tools. In addition to workspace/execution/network/LSP tools, it includes bounded local Git mutation, credential-isolated remote Git, and forge-neutral change-request operations. If the deployed relay has not yet been upgraded to the Plan-040 artifact, live external MCP client discovery may still show the previous catalog until deployment and connector refresh.

```text
terminal_exec
http_fetch
web_search
directory_list
file_search
file_write
file_edit
file_read
text_search
git_status
git_diff
git_log
git_show
git_blame
git_branch_list
git_branch_create
git_branch_switch
git_stage
git_unstage
git_commit
git_operation_status
git_merge_start
git_merge_continue
git_merge_abort
git_rebase_start
git_rebase_continue
git_rebase_abort
git_branch_delete
git_remote_list
git_remote_branch_get
git_fetch
git_push
git_remote_branch_delete
change_request_list
change_request_get
change_request_create
change_request_update
change_request_checks
change_request_merge
issue_list
issue_get
issue_create
issue_update
issue_comment
issue_close
issue_reopen
workflow_list
workflow_run_list
workflow_run_get
workflow_job_get
workflow_run_job_log
apply_patch
code_symbols
code_definition
code_references
code_implementations
code_hover
code_diagnostics
code_rename_preview
terminal_job_start
terminal_job_get
terminal_job_cancel
```

The native workspace contracts are:

```text
directory_list(path=".", cwd?, depth=2, max_entries=100)
file_search(pattern, cwd?, max_results=100)
text_search(query, cwd?, glob?, regex=false, case_sensitive=true, max_results=50)
file_read(path, cwd?, offset_line=1, limit_lines=200)
file_edit(path, old_text, new_text, cwd?, replace_all=false)
file_write(path, content, cwd?, create_parents=false, overwrite=false)
apply_patch(patch, cwd?, dry_run=false)
git_status(cwd?, include_untracked=true)
git_diff(cwd?, mode=working|staged|refs, base_ref?, head_ref?, path?, context_lines?, max_bytes?)
git_log(cwd?, ref?, path?, max_results?)
git_show(cwd?, ref, path?, include_patch=true, max_bytes?)
git_blame(cwd?, path, start_line?, end_line?)
git_branch_list(cwd?)
git_branch_create(cwd?, name, start_point?)
git_branch_switch(cwd?, name)
git_stage(cwd?, paths[])
git_unstage(cwd?, paths[])
git_commit(cwd?, message)
git_operation_status(cwd?)
git_merge_start(cwd?, ref)
git_merge_continue(cwd?)
git_merge_abort(cwd?)
git_rebase_start(cwd?, ref)
git_rebase_continue(cwd?)
git_rebase_abort(cwd?)
git_branch_delete(cwd?, name)
git_remote_list(cwd?)
git_remote_branch_get(cwd?, remote="origin", branch)
git_fetch(cwd?, remote="origin", branch)
git_push(cwd?, remote="origin", branch, set_upstream=false)
git_remote_branch_delete(cwd?, remote="origin", branch, expected_sha)
change_request_list(cwd?, remote="origin", state="open")
change_request_get(cwd?, remote="origin", number)
change_request_create(cwd?, remote="origin", head_branch, base_branch, title, body, draft=false)
change_request_update(cwd?, remote="origin", number, title?, body?, base_branch?)
change_request_checks(cwd?, remote="origin", number)
change_request_merge(cwd?, remote="origin", number, expected_head_sha, strategy="squash")
issue_list(cwd?, remote="origin", state="open", labels=[])
issue_get(cwd?, remote="origin", number)
issue_create(cwd?, remote="origin", title, body="", labels=[])
issue_update(cwd?, remote="origin", number, title?, body?, add_labels=[], remove_labels=[])
issue_comment(cwd?, remote="origin", number, body)
issue_close(cwd?, remote="origin", number, reason="completed"|"not_planned"|"duplicate", duplicate_of?, comment?)
issue_reopen(cwd?, remote="origin", number, comment?)
workflow_list(cwd?, remote="origin")
workflow_run_list(cwd?, remote="origin", workflow?, branch?, status?)
workflow_run_get(cwd?, remote="origin", number)
workflow_job_get(cwd?, remote="origin", number)
workflow_run_job_log(cwd?, remote="origin", number, job_id?, max_lines=100)
code_symbols(path=..., cwd?, max_results?) or code_symbols(query=..., cwd?, max_results?)
code_definition(...)
code_references(...)
code_implementations(...)
code_hover(...)
code_diagnostics(...)
code_rename_preview(...)
```

Server hard limits remain authoritative even when a caller supplies its own limit. `directory_list` caps depth at 4 and returned entries at 100; `file_search` and `text_search` cap returned matches at 100; `file_read` caps a request at 1,000 lines and 256 KiB; `file_edit` and `file_write` cap file/payload content at 1 MiB. Mutation defaults are deliberately conservative: an ambiguous `file_edit` fails, and `file_write` never replaces an existing file unless `overwrite=true`.

Use native Git tools for status/diff/history plus bounded branch/stage/commit/merge/rebase/conflict workflows, remote-Git tools for validated fetch/push/remote-branch operations, `change_request_*` for the GitHub PR lifecycle through the forge-neutral contract, the `code_*` tools for bounded language intelligence/diagnostics, and `apply_patch` for bounded multi-hunk existing-file changes. `code_symbols(query=...)` uses the configured language server's workspace-symbol capability for Rust and TypeScript-family workspaces; unsupported server capabilities fail explicitly rather than falling back to a custom index. Vue document symbols are supported with the reviewed Vue bridge, while Vue definition/references/hover/diagnostics remain bounded-empty with the currently reviewed `@vue/language-server` build and must not be presented as semantic parity. Keep `terminal_exec` for builds, tests, package managers, interpreters, project scripts, and unsupported operations; ordinary terminal execution remains credential-isolated and is not the GitHub delivery bridge. Terminal arguments use direct argv semantics: values beginning with `-` or `--` are valid child-process arguments (for example `command="cargo", args=["--help"]` or `args=["check", "--locked"]`). The `command` executable must resolve from the relay safe PATH; invoke repository scripts through an approved interpreter, for example `command="bash", args=["scripts/check.sh"]`, rather than using `./scripts/check.sh` as the command. `terminal_exec` also supports the current MCP task lifecycle for clients that negotiate it; the `terminal_job_*` tools use the same argv contract and are the explicit polling/cancellation fallback for first-party or non-Tasks clients.

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
