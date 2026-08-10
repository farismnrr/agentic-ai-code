# Local CLI Relay Agent (`@ai-code/relay-agent`)

The `@ai-code/relay-agent` package provides a secure, loopback-only terminal runner that binds exclusively to `127.0.0.1` on your local laptop or machine.

## Purpose

It acts as a local bridge between the AI Code web interface and your machine's local shell. Terminal command execution stays entirely on your computer — no terminal traffic or shell bytes leave over the internet.

## Usage

### 1. Installation & Execution

Run the CLI in your desired workspace folder:

```bash
npx @ai-code/relay-agent start --dir ./my-project
```

Options:
- `--dir, -d`: Root directory for the scoped workspace (defaults to current working directory).
- `--port, -p`: Local HTTP/WebSocket port to bind (defaults to `47821`).

### 2. Pairing with Web UI

1. The CLI prints a one-time pairing token upon startup.
2. Go to **Settings > Local Terminal** in the web interface.
3. Paste the pairing token and click **Pair**.
4. The web UI issues a long-lived session credential saved in your browser (`localStorage`).
