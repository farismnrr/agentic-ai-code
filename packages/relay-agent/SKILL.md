# Local CLI Relay Agent (`@ai-code/relay-agent`)

The `@ai-code/relay-agent` package provides a secure, loopback-only terminal runner that binds exclusively to `127.0.0.1` on your local laptop or machine.

## Purpose

It acts as a local bridge between the AI Code web interface and your machine's local shell. Terminal command execution stays entirely on your computer — no terminal traffic or shell bytes leave over the internet.

**This agent has no directory jail.** `--dir` sets only the *default* starting directory for a command that doesn't specify its own `cwd` — it is not a boundary, and a command (or the `cwd` a caller sends) can target anywhere this OS user account can reach. This was a deliberate choice (see `.agents/plans/026-local-cli-relay-agent.md`'s Phase 8 note) over the original workspace-scoped design. The actual controls are: (1) pairing — nothing can reach this agent at all without a valid session credential from a successful pairing, and (2) the per-command approval gate in the chat UI for anything the AI initiates (manual commands typed into the paired browser's own terminal panel need no approval — same trust level as opening a real terminal). Run this agent under an OS account whose reach you're comfortable with a paired session having in full.

## Usage

### 1. Installation & Execution

`packages/relay-agent` is `"private": true` — it is never published to npm, so `npx @ai-code/relay-agent` only works from inside this monorepo (pnpm workspace resolution). End users get it as a **standalone compiled binary** instead (see "Building standalone binaries" below); from inside this repo, run it via the workspace script:

```bash
node packages/relay-agent/bin/cli.mjs --dir ./my-project --origin http://localhost:3333
```

or, once compiled, directly:

```bash
./relay-agent-linux-x64 --dir ./my-project --origin http://localhost:3333
```

Options:
- `--dir, -d`: Default starting directory for commands that don't specify their own `cwd` — **not a restriction** (defaults to your home directory, not the current working directory; see the no-jail note above).
- `--port, -p`: Local HTTP/WebSocket port to bind (defaults to `47821`).
- `--origin, -o`: The web app's exact origin (scheme + host + port) — required for anything other than this repo's own local dev server. The CLI runs on the user's machine as its own process; it cannot read the web app's `runtimeConfig.public.siteUrl`/`NUXT_PUBLIC_SITE_URL`, so it never guesses a real deployment's origin — the fallback (`http://localhost:3333`, `RelayAgentServer`'s one default, in `src/server.ts`) only matches this repo's own documented dev port and exists purely for local-dev convenience. Can also be set once via the `RELAY_AGENT_ORIGIN` env var instead of passing `--origin` every run. The web app's own Settings → Local Terminal page always shows the exact value to pass for wherever it's currently being viewed from.

### Stopping it

`Ctrl+C` (SIGINT) in the foreground terminal shuts it down cleanly — closes the WebSocket/HTTP server and exits, rather than just being killed outright.

If it's running detached/in the background (or you don't have the terminal it was started from), use the `stop` subcommand instead of hunting for the PID yourself:

```bash
./relay-agent-linux-x64 stop --port 47821   # --port only needed if not the default
```

This works via a pidfile (`bin/pidfile.mjs`) written to `os.tmpdir()/relay-agent-<port>.pid` on start and removed on clean shutdown — `stop` reads it, sends `SIGTERM` to that pid, and reports if nothing was actually running (including cleaning up a stale pidfile left behind by a hard-killed/crashed process, so a later `stop` doesn't get confused by it).


### 2. Pairing with Web UI

1. The CLI prints a one-time pairing token upon startup.
2. Go to **Settings > Local Terminal** in the web interface.
3. Paste the pairing token and click **Pair**.
4. The web UI issues a long-lived session credential saved in your browser (`localStorage`).
