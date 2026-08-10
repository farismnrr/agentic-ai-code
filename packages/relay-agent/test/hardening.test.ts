import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import path from 'node:path'
import { WebSocket } from 'ws'
import { RelayAgentServer } from '../src/index.ts'

interface ExecResultPayload {
  type: string
  id?: string
  success: boolean
  error?: string
  stdout?: string
  stderr?: string
  exitCode?: number
}

describe('RelayAgent Phase 2 - Pairing & Hardening', () => {
  const testDir = path.resolve(process.cwd())
  let server: RelayAgentServer
  const testPort = 47835
  const allowedOrigin = 'http://localhost:3000'

  beforeAll(async () => {
    server = new RelayAgentServer({
      port: testPort,
      dir: testDir,
      allowedOrigin
    })
    await server.start()
  })

  afterAll(async () => {
    await server.stop()
  })

  it('rejects HTTP request with invalid Origin', async () => {
    const res = await fetch(`http://127.0.0.1:${testPort}/health`, {
      headers: { Origin: 'https://evil.example' }
    })
    expect(res.status).toBe(403)
    const json = await res.json()
    expect(json.error).toContain('Disallowed Origin')
  })

  it('rejects HTTP pairing request with invalid token', async () => {
    const res = await fetch(`http://127.0.0.1:${testPort}/pair`, {
      method: 'POST',
      headers: {
        'Origin': allowedOrigin,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ token: 'wrongtoken' })
    })
    expect(res.status).toBe(401)
  })

  it('pairs successfully with valid token and issues session credential', async () => {
    const initialToken = server.pairingToken
    const res = await fetch(`http://127.0.0.1:${testPort}/pair`, {
      method: 'POST',
      headers: {
        'Origin': allowedOrigin,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ token: initialToken })
    })
    expect(res.status).toBe(200)
    const json = await res.json()
    expect(json.sessionCredential).toBeDefined()
    expect(typeof json.sessionCredential).toBe('string')

    // Confirm pairing token is single use and now revoked
    const retryRes = await fetch(`http://127.0.0.1:${testPort}/pair`, {
      method: 'POST',
      headers: {
        'Origin': allowedOrigin,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ token: initialToken })
    })
    expect(retryRes.status).toBe(401)
  })

  it('connects WebSocket via issued session credential', async () => {
    // Generate new token manually or trigger pairing
    server.pairingToken = 'test-pairing-token'
    const pairRes = await fetch(`http://127.0.0.1:${testPort}/pair`, {
      method: 'POST',
      headers: {
        'Origin': allowedOrigin,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ token: 'test-pairing-token' })
    })
    const { sessionCredential } = await pairRes.json()

    const ws = new WebSocket(`ws://127.0.0.1:${testPort}?credential=${sessionCredential}`, {
      headers: { Origin: allowedOrigin }
    })

    await new Promise<void>((resolve, reject) => {
      ws.on('open', () => resolve())
      ws.on('error', reject)
    })

    const response = await new Promise<ExecResultPayload>((resolve) => {
      ws.on('message', (data) => {
        resolve(JSON.parse(data.toString()) as ExecResultPayload)
      })
      ws.send(JSON.stringify({ type: 'exec', id: 'p2-1', command: 'echo paired' }))
    })

    expect(response.success).toBe(true)
    expect(response.stdout?.trim()).toBe('paired')
    ws.close()
  })

  it('rejects WebSocket connection with invalid credential or token', async () => {
    const ws = new WebSocket(`ws://127.0.0.1:${testPort}?credential=fakecred`, {
      headers: { Origin: allowedOrigin }
    })

    const failed = await new Promise<boolean>((resolve) => {
      ws.on('error', () => resolve(true))
      ws.on('open', () => {
        ws.close()
        resolve(false)
      })
    })

    expect(failed).toBe(true)
  })
})
