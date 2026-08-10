import { createServer, type Server as HttpServer, type IncomingMessage, type ServerResponse } from 'node:http'
import path from 'node:path'
import crypto from 'node:crypto'
import { WebSocketServer, type WebSocket } from 'ws'
import { execa } from 'execa'
import { resolveScopedPath } from './scope.ts'

export interface RelayAgentOptions {
  port?: number
  dir?: string
  allowedOrigin?: string
}

export class RelayAgentServer {
  public readonly port: number
  public readonly workspaceDir: string
  public readonly allowedOrigin: string
  public pairingToken: string
  private sessionCredentials: Set<string> = new Set()
  private httpServer: HttpServer | null = null
  private wss: WebSocketServer | null = null

  constructor(options: RelayAgentOptions = {}) {
    this.port = options.port ?? 47821
    this.workspaceDir = path.resolve(options.dir ?? process.cwd())
    this.allowedOrigin = options.allowedOrigin ?? 'http://localhost:3000'
    this.pairingToken = crypto.randomBytes(16).toString('hex')
  }

  private validateHostAndOrigin(req: IncomingMessage): { valid: boolean, reason?: string } {
    const host = req.headers.host
    const origin = req.headers.origin

    const expectedHosts = [`127.0.0.1:${this.port}`, `localhost:${this.port}`]
    if (!host || !expectedHosts.includes(host)) {
      return { valid: false, reason: `Invalid Host header: ${host}` }
    }

    if (origin && origin !== this.allowedOrigin) {
      return { valid: false, reason: `Disallowed Origin header: ${origin}` }
    }

    return { valid: true }
  }

  public async start(): Promise<void> {
    await resolveScopedPath('.', this.workspaceDir)

    this.httpServer = createServer((req: IncomingMessage, res: ServerResponse) => {
      const validation = this.validateHostAndOrigin(req)
      if (!validation.valid) {
        res.writeHead(403, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({ error: validation.reason }))
        return
      }

      res.setHeader('Access-Control-Allow-Origin', this.allowedOrigin)
      res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
      res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization')

      if (req.method === 'OPTIONS') {
        res.writeHead(204)
        res.end()
        return
      }

      const url = new URL(req.url ?? '/', `http://${req.headers.host}`)

      if (req.method === 'POST' && url.pathname === '/pair') {
        let body = ''
        req.on('data', (chunk) => {
          body += chunk
        })
        req.on('end', () => {
          try {
            const data = JSON.parse(body || '{}')
            if (!data.token || data.token !== this.pairingToken) {
              res.writeHead(401, { 'Content-Type': 'application/json' })
              res.end(JSON.stringify({ error: 'Invalid or expired pairing token' }))
              return
            }

            // Single use pairing token: generate session credential
            const sessionCred = crypto.randomBytes(32).toString('hex')
            this.sessionCredentials.add(sessionCred)
            // Revoke pairing token after single use
            this.pairingToken = ''

            res.writeHead(200, { 'Content-Type': 'application/json' })
            res.end(JSON.stringify({ sessionCredential: sessionCred }))
          } catch (err: unknown) {
            res.writeHead(400, { 'Content-Type': 'application/json' })
            res.end(JSON.stringify({ error: (err as Error).message }))
          }
        })
        return
      }

      if (req.method === 'GET' && url.pathname === '/health') {
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({ status: 'ok', agent: 'relay-agent', workspace: this.workspaceDir }))
        return
      }

      res.writeHead(404, { 'Content-Type': 'application/json' })
      res.end(JSON.stringify({ error: 'Not found' }))
    })

    this.wss = new WebSocketServer({ noServer: true })

    this.httpServer.on('upgrade', (request, socket, head) => {
      const validation = this.validateHostAndOrigin(request)
      if (!validation.valid) {
        socket.write('HTTP/1.1 403 Forbidden\r\n\r\n')
        socket.destroy()
        return
      }

      const url = new URL(request.url ?? '/', `http://${request.headers.host}`)
      const token = url.searchParams.get('token')
      const cred = url.searchParams.get('credential')

      let authorized = false
      if (cred && this.sessionCredentials.has(cred)) {
        authorized = true
      } else if (token && this.pairingToken && token === this.pairingToken) {
        // Upgrade via direct pairing token generates a session credential
        authorized = true
      }

      if (!authorized) {
        socket.write('HTTP/1.1 401 Unauthorized\r\n\r\n')
        socket.destroy()
        return
      }

      this.wss?.handleUpgrade(request, socket, head, (ws) => {
        this.wss?.emit('connection', ws, request)
      })
    })

    this.wss.on('connection', (ws: WebSocket) => {
      ws.on('message', async (data: Buffer | string) => {
        try {
          const payload = JSON.parse(data.toString())
          if (payload.type === 'exec') {
            await this.handleExecCommand(ws, payload)
          } else {
            ws.send(JSON.stringify({ type: 'error', error: `Unknown message type: ${payload.type}` }))
          }
        } catch (err: unknown) {
          ws.send(JSON.stringify({ type: 'error', error: (err as Error).message }))
        }
      })
    })

    return new Promise((resolve, reject) => {
      this.httpServer?.listen(this.port, '127.0.0.1', () => {
        console.log(`[relay-agent] Listening on http://127.0.0.1:${this.port}`)
        console.log(`[relay-agent] Pairing token: ${this.pairingToken}`)
        console.log(`[relay-agent] Workspace directory: ${this.workspaceDir}`)
        resolve()
      })
      this.httpServer?.on('error', reject)
    })
  }

  private async handleExecCommand(ws: WebSocket, payload: { id?: string, command?: string, args?: string[], cwd?: string }): Promise<void> {
    const { id, command, args = [], cwd } = payload
    if (!command) {
      ws.send(JSON.stringify({ type: 'exec_result', id, success: false, error: 'Command is required' }))
      return
    }

    try {
      const targetCwd = cwd ? await resolveScopedPath(cwd, this.workspaceDir) : this.workspaceDir

      const [binary, ...gluedArgs] = command.trim().split(/\s+/)
      const finalCommand = binary ?? command
      const finalArgs = [...gluedArgs, ...args]

      if (finalCommand.includes('/') || finalCommand.includes('\\')) {
        await resolveScopedPath(finalCommand, this.workspaceDir)
      }

      const env: Record<string, string> = {}
      if (process.env.PATH) env.PATH = process.env.PATH
      if (process.env.HOME) env.HOME = process.env.HOME
      if (process.env.LANG) env.LANG = process.env.LANG

      const timeoutMs = 30000
      const result = await execa(finalCommand, finalArgs, {
        shell: false,
        cwd: targetCwd,
        env,
        extendEnv: false,
        timeout: timeoutMs,
        killSignal: 'SIGKILL',
        reject: false
      })

      if (result.timedOut) {
        ws.send(JSON.stringify({
          type: 'exec_result',
          id,
          success: false,
          error: `Command timed out after ${timeoutMs / 1000}s`,
          stdout: result.stdout,
          stderr: result.stderr
        }))
        return
      }

      if (result.failed) {
        ws.send(JSON.stringify({
          type: 'exec_result',
          id,
          success: false,
          error: result.shortMessage,
          exitCode: result.exitCode,
          stdout: result.stdout,
          stderr: result.stderr
        }))
        return
      }

      ws.send(JSON.stringify({
        type: 'exec_result',
        id,
        success: true,
        exitCode: result.exitCode,
        stdout: result.stdout,
        stderr: result.stderr
      }))
    } catch (err: unknown) {
      ws.send(JSON.stringify({
        type: 'exec_result',
        id,
        success: false,
        error: (err as Error).message
      }))
    }
  }

  public async stop(): Promise<void> {
    return new Promise((resolve) => {
      this.wss?.close()
      this.httpServer?.close(() => {
        resolve()
      })
    })
  }
}
