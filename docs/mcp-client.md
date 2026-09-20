# Connect a compatible MCP client

Any MCP client that supports Streamable HTTP and the standard OAuth protected-
resource flow can connect to the relay. The relay does not require a
client-specific transport or UI integration.

Before connecting, complete:

1. [OAuth/OIDC Authorization Server setup](oauth-provider.md);
2. [Remote MCP deployment](remote-mcp.md);
3. the unauthenticated public smoke checks; and
4. preferably an authenticated owner-token `server/discover` + `tools/list`
   smoke check.

## 1. Use the public MCP resource URL

Configure the client with the canonical HTTPS resource URL, for example:

```text
https://mcp.example.com/mcp
```

Do not use the relay's loopback address from a remote client. The local relay
may remain bound to `127.0.0.1`; a tunnel or reverse proxy publishes the
public URL and forwards only to that loopback listener.

## 2. Configure OAuth in the client

Choose OAuth/OIDC when the client asks how to authenticate. The client should
discover the relay's Protected Resource Metadata, follow the advertised
Authorization Server, use authorization code + PKCE S256, and request the
scope `relay.coding`.

If the client asks for a redirect/callback URI, register that exact value in
the Authorization Server. Never allow a wildcard redirect or reuse a callback
from another environment. Client registration may be pre-created, supplied by
a Client ID Metadata Document, or enabled through tightly restricted dynamic
registration, depending on the client and Authorization Server.

Authenticate as the external identity whose stable `sub` claim is configured
as `OAUTH_OWNER_SUBJECT`. The resulting token must have:

- `iss` equal to `OAUTH_ISSUER`;
- `aud` containing the exact MCP resource URL;
- a valid asymmetric signature discoverable through OIDC JWKS;
- valid time claims; and
- the `relay.coding` scope.

Refresh-token support is recommended for long-lived connections. Request
`offline_access` only when the Authorization Server and client support it;
it controls token renewal and is not a substitute for the relay permission
scope.

## 3. Discover and verify tools

The Full profile is the canonical superset. A deployment using the Primary
profile advertises a smaller intentional subset. In the Full profile, the
delegated coding tool is further filtered to the locally authenticated CLI
sessions discovered at relay startup; unavailable providers are not advertised
or accepted in the provider schema. Restart the relay after changing a local
CLI login. Tool visibility does not weaken OAuth, workspace containment, or
relay policy.

Start with read-only operations such as `directory_list`, `file_search`,
`file_read`, `text_search`, or `directory_list`. Then use a small bounded command
and inspect its result before approving higher-risk operations. Client-side
confirmation is useful UX, but the relay remains the server-side authority.

## 4. Editing large files safely

The native mutation tools are incremental and bounded:

```text
file_edit(path, old_text, new_text, cwd?, replace_all=false)
file_edit(path, edits=[{old_text, new_text, replace_all?}, ...], cwd?)
apply_patch(patch, cwd?, dry_run=false)
file_write(path, content, cwd?, create_parents=false, overwrite=false)
```

Use `file_read` with a line range to retrieve the relevant context from a
large file. Use one `file_edit` anchor for a single change, or `edits[]` for
several independent changes in one file. Every anchor is matched against the
original snapshot; missing, ambiguous, or overlapping anchors fail before any
write. The result is committed once with the original file identity and mode
preserved.

Use `apply_patch` for multi-hunk or multi-file changes. It validates every
target and hunk before an atomic per-file commit with bounded rollback. Use
`file_write` for creating a file or intentionally replacing a complete small
file—not as the normal way to rewrite thousands of lines.

Example incremental edit:

```json
{
  "path": "src/service.ts",
  "edits": [
    {
      "old_text": "const timeoutMs = 5000;",
      "new_text": "const timeoutMs = 15000;"
    },
    {
      "old_text": "return legacyResult(value);",
      "new_text": "return normalizedResult(value);"
    }
  ]
}
```

Anchors should include enough surrounding context to be unique and stable.
Do not use `replace_all` for a broad token unless replacing every occurrence is
intentional.

## 5. Automatic bootstrap guidance

The current Masih Awam relay source advertises a bounded universal bootstrap through `server/discover.instructions` and the legacy-compatible `initialize.instructions`. That instruction covers fresh workspace verification, approved repository guidance, the shared Task Execution Report structure, truthful verification/delivery reporting, and foreground handoff for work that cannot finish inside a public tool deadline.

The server-owned `workspace://<repo>/agent-guidance` resource separately exposes the verified repository's `ai-self/BOOTSTRAP.md` when present, followed by `AGENTS.md` and `.agents/knowledge/resources.md`. The first-party Nuxt modern-MCP path explicitly promotes only `first-party-relay` discovered instructions into top-level chat and delegated-subagent system context; instructions supplied by external/third-party MCP servers are not promoted by this mechanism.

An arbitrary external MCP client may ignore MCP server instructions or resources. Live ChatGPT connector acceptance confirmed that `server/discover.instructions` alone is not sufficient for every host, so the relay also appends the compact reporting contract to its MCP wire tool descriptions without changing internal catalog definitions or structured tool-result envelopes. This is compatibility reinforcement, not a guarantee that an external host must obey server guidance. If a client also ignores tool descriptions, copy the repository's `ai-self/BOOTSTRAP.md` into that client's global instructions as the final fallback. Do not duplicate the bootstrap into each project when the client already honors the relay-provided contract.

## 6. Slow operations and tasks

`terminal_exec` is synchronous-only. Its timeout is `1..=60000` milliseconds
with a 30 second default. It does not expose MCP Tasks or terminal job polling
tools. Commands expected to exceed 60 seconds must be run manually by the
operator instead of being handed off to background execution.

The retained process-like coding tools (`terminal_exec`, `ssh_readonly_exec`,
`http_fetch`, and `web_search`) are synchronous-only at the MCP boundary.
Where a tool exposes `timeout_ms`, its public range is `1..=60000`.

## 7. Execution contract

Retained coding tools do not accept `execution_mode`. They return a bounded
direct result or a timeout/error result; work that cannot finish within the
bounded call must be handed back to the operator instead of detached.

## What a successful connection proves

OAuth discovery, `tools/list`, and one safe call prove that this client can
reach and use the configured relay. They do not prove every tool's negative
case, the hosted application's ownership path, or private token-claim
internals. Keep those acceptance claims separate.
