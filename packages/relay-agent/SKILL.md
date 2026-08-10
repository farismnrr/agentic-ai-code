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

### Building standalone binaries

End users should not need Node.js/npm installed. `packages/relay-agent` compiles to a single self-contained executable per platform:

```bash
pnpm --filter @ai-code/relay-agent run build    # esbuild: bin/cli.mjs + src/* → dist/bundle.cjs (one bundled CJS file)
pnpm --filter @ai-code/relay-agent run compile  # build.mjs, then @yao-pkg/pkg: dist/bundle.cjs → dist/bin/bundle-{linux-x64,macos-x64,macos-arm64,win-x64.exe}
```

`@yao-pkg/pkg` (a maintained fork of Vercel's now-unmaintained `pkg`) *can* cross-build all four targets from one Linux CI runner with no macOS/Windows build machines needed — but only when its remote cache actually has a prebuilt Node binary for the exact `<node-major>-<platform>-<arch>` combination requested. When it doesn't, it silently falls back to compiling Node.js from source, which is a 20–60+ minute `./configure && make` per target on `linux`, and an outright hard failure for `macos`/`win` targets specifically (`Error! Not able to build for 'macos' here, only for 'linux'` — cross-OS source builds aren't supported at all, only cross-OS *prebuilt-binary* fetches are).

**Verified by trial**: `node18`/`node20` currently 404 out of `@yao-pkg/pkg`'s remote cache (confirmed both by a real `compile` run and by probing individual targets with `pkg-fetch -t`, e.g. `node node_modules/.pnpm/@yao-pkg+pkg-fetch@*/node_modules/@yao-pkg/pkg-fetch/lib-es5/bin.js -n node20 -p linux -a x64 -t`). `node22` and `node24` do have full 4-platform cache coverage as of this writing (all four fetch successfully, confirmed by running `compile` end-to-end and checking `file` on each output — real ELF/Mach-O×2/PE32+ binaries, not from-source rebuilds). `package.json`'s `compile` script is pinned to `node22` for this reason — **if bumping this later, re-verify with the `pkg-fetch -t` probe above across all four targets before changing `package.json`, don't assume the newest node alias has cache coverage.**

This is why `.github/workflows/release-relay-agent.yml` (triggered on `relay-agent-v*` tags) can run entirely on `ubuntu-latest`: it runs `compile`, renames the four `dist/bin/bundle-*` outputs to the fixed asset names (`relay-agent-linux-x64`, `relay-agent-macos-x64`, `relay-agent-macos-arm64`, `relay-agent-win-x64.exe` — note pkg's real output names are `bundle-<platform>-<arch>`, e.g. `bundle-linux-x64`/`bundle-win-x64.exe`, not `bundle-linux`/`bundle-win.exe`), and uploads them to a GitHub Release. The web UI's download buttons link directly to `github.com/<org>/<repo>/releases/latest/download/<asset-name>` — that URL form always resolves to the current release's matching asset, so the download links never need updating when a new version ships; only the fixed asset names must never change without updating both the workflow and the UI together.

The binaries are unsigned (no Apple notarization, no Windows code-signing cert configured) — macOS Gatekeeper and Windows SmartScreen will warn on first run. The settings page documents the workaround (right-click → Open, or `xattr -d com.apple.quarantine`) next to the download buttons.

### 2. Pairing with Web UI

1. The CLI prints a one-time pairing token upon startup.
2. Go to **Settings > Local Terminal** in the web interface.
3. Paste the pairing token and click **Pair**.
4. The web UI issues a long-lived session credential saved in your browser (`localStorage`).
