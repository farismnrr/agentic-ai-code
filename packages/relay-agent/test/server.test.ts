import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import path from 'node:path'
import { WebSocket } from 'ws'
import { RelayAgentServer } from '../src/index.ts'
import { resolveScopedPath } from '../src/scope.ts'

interface ExecResultPayload {
  type: string
  id?: string
  success: boolean
  error?: string
  stdout?: string
  stderr?: string
  exitCode?: number
}

describe('RelayAgent - Scope & Core WS', () => {
  const testDir = path.resolve(process.cwd())
  let server: RelayAgentServer
  const testPort = 47830
  const allowedOrigin = 'http://localhost:3000'

  beforeAll(async () => {
    server = new RelayAgentServer({ port: testPort, dir: testDir, allowedOrigin })
    await server.start()
  })

  afterAll(async () => {
    await server.stop()
  })

  it('allows valid in-scope paths', async () => {
    const resolved = await resolveScopedPath('.', testDir)
    expect(resolved).toBe(testDir)
  })

  it('rejects path traversal attempts', async () => {
    await expect(resolveScopedPath('../../..', testDir)).rejects.toThrow('Path traversal blocked')
  })

  it('connects over WebSocket via session credential and executes in-scope commands', async () => {
    // Pair first
    const pairRes = await fetch(`http://127.0.0.1:${testPort}/pair`, {
      method: 'POST',
      headers: {
        'Origin': allowedOrigin,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ token: server.pairingToken })
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
      ws.send(JSON.stringify({ type: 'exec', id: '1', command: 'echo hello' }))
    })

    expect(response.type).toBe('exec_result')
    expect(response.id).toBe('1')
    expect(response.success).toBe(true)
    expect(response.stdout?.trim()).toBe('hello')

    ws.close()
  })
})
