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

describe('RelayAgent Hardening & Revocation', () => {
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

  it('rejects HTTP request with missing or invalid Origin', async () => {
    const resNoOrigin = await fetch(`http://127.0.0.1:${testPort}/health`)
    expect(resNoOrigin.status).toBe(403)

    const resEvilOrigin = await fetch(`http://127.0.0.1:${testPort}/health`, {
      headers: { Origin: 'https://evil.example' }
    })
    expect(resEvilOrigin.status).toBe(403)
  })

  it('rejects pairing with expired token', async () => {
    const expiredServer = new RelayAgentServer({
      port: testPort + 1,
      dir: testDir,
      allowedOrigin,
      tokenTtlMs: -1000 // already expired
    })
    await expiredServer.start()

    const res = await fetch(`http://127.0.0.1:${testPort + 1}/pair`, {
      method: 'POST',
      headers: {
        'Origin': allowedOrigin,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ token: expiredServer.pairingToken })
    })
    expect(res.status).toBe(401)
    const json = await res.json()
    expect(json.error).toContain('expired')

    await expiredServer.stop()
  })

  it('allows pairing with valid token and revokes token after use', async () => {
    const token = server.pairingToken
    const res = await fetch(`http://127.0.0.1:${testPort}/pair`, {
      method: 'POST',
      headers: {
        'Origin': allowedOrigin,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ token })
    })
    expect(res.status).toBe(200)
    const { sessionCredential } = await res.json()
    expect(sessionCredential).toBeDefined()

    // Second use of same token fails
    const res2 = await fetch(`http://127.0.0.1:${testPort}/pair`, {
      method: 'POST',
      headers: {
        'Origin': allowedOrigin,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ token })
    })
    expect(res2.status).toBe(401)
  })

  it('revokes session credential via /revoke endpoint', async () => {
    server.pairingToken = 'test-revoke-token'
    server.pairingTokenExpiresAt = Date.now() + 60000

    const pairRes = await fetch(`http://127.0.0.1:${testPort}/pair`, {
      method: 'POST',
      headers: {
        'Origin': allowedOrigin,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ token: 'test-revoke-token' })
    })
    const { sessionCredential } = await pairRes.json()

    // Test credential works
    const ws = new WebSocket(`ws://127.0.0.1:${testPort}?credential=${sessionCredential}`, {
      headers: { Origin: allowedOrigin }
    })
    await new Promise<void>((resolve, reject) => {
      ws.on('open', () => resolve())
      ws.on('error', reject)
    })
    ws.close()

    // Call /revoke
    const revokeRes = await fetch(`http://127.0.0.1:${testPort}/revoke`, {
      method: 'POST',
      headers: {
        'Origin': allowedOrigin,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ credential: sessionCredential })
    })
    expect(revokeRes.status).toBe(200)

    // Verify connection is rejected after revocation
    const wsRevoked = new WebSocket(`ws://127.0.0.1:${testPort}?credential=${sessionCredential}`, {
      headers: { Origin: allowedOrigin }
    })
    const failed = await new Promise<boolean>((resolve) => {
      wsRevoked.on('error', () => resolve(true))
      wsRevoked.on('open', () => {
        wsRevoked.close()
        resolve(false)
      })
    })
    expect(failed).toBe(true)
  })

  it('blocks path traversal in command arguments', async () => {
    server.pairingToken = 'test-arg-token'
    server.pairingTokenExpiresAt = Date.now() + 60000

    const pairRes = await fetch(`http://127.0.0.1:${testPort}/pair`, {
      method: 'POST',
      headers: {
        'Origin': allowedOrigin,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ token: 'test-arg-token' })
    })
    const { sessionCredential } = await pairRes.json()

    const ws = new WebSocket(`ws://127.0.0.1:${testPort}?credential=${sessionCredential}`, {
      headers: { Origin: allowedOrigin }
    })
    await new Promise<void>(resolve => ws.on('open', () => resolve()))

    const response = await new Promise<ExecResultPayload>((resolve) => {
      ws.on('message', data => resolve(JSON.parse(data.toString()) as ExecResultPayload))
      ws.send(JSON.stringify({ type: 'exec', id: 'traversal-1', command: 'cat', args: ['../../../etc/passwd'] }))
    })

    expect(response.success).toBe(false)
    expect(response.error).toContain('Path traversal blocked')
    ws.close()
  })
})
