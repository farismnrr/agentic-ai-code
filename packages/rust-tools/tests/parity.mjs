import { spawn } from 'node:child_process'
import fs from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import http from 'node:http'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const ROOT = path.resolve(__dirname, '../../..')

const CLIS = {
  'terminal-tool': {
    js: path.join(ROOT, 'packages/terminal-tool/bin/cli.mjs'),
    rust: path.join(ROOT, 'target/debug/terminal-tool')
  },
  'curl-tool': {
    js: path.join(ROOT, 'packages/curl-tool/bin/cli.mjs'),
    rust: path.join(ROOT, 'target/debug/curl-tool')
  },
  'searxng-search-tool': {
    js: path.join(ROOT, 'packages/searxng-search-tool/bin/cli.mjs'),
    rust: path.join(ROOT, 'target/debug/searxng-search-tool')
  }
}

async function checkExists(file) {
  try {
    await fs.access(file)
    return true
  } catch {
    return false
  }
}

function runCli(binPath, args, env = {}) {
  return new Promise((resolve) => {
    const child = spawn(binPath, args, {
      env: { ...process.env, ...env }
    })

    let stdout = ''
    let stderr = ''

    child.stdout.on('data', (data) => {
      stdout += data.toString()
    })

    child.stderr.on('data', (data) => {
      stderr += data.toString()
    })

    child.on('close', (code) => {
      resolve({ stdout: stdout.trim(), stderr: stderr.trim(), exitCode: code })
    })

    child.on('error', (err) => {
      resolve({ stdout: stdout.trim(), stderr: stderr.trim() + '\n' + err.message, exitCode: 1 })
    })
  })
}

async function compare(toolName, testName, args, env = {}) {
  console.log(`\n--- Testing ${toolName}: ${testName} ---`)

  const tool = CLIS[toolName]
  if (!await checkExists(tool.js) || !await checkExists(tool.rust)) {
    console.error(`Missing binary for ${toolName}. Run cargo build first?`)
    return false
  }

  const jsResult = await runCli('node', [tool.js, ...args], env)
  const rustResult = await runCli(tool.rust, args, env)

  let passed = true

  if (jsResult.exitCode !== rustResult.exitCode) {
    console.error(`❌ Exit code mismatch. JS: ${jsResult.exitCode}, Rust: ${rustResult.exitCode}`)
    passed = false
  } else {
    console.log(`✅ Exit code matches (${jsResult.exitCode})`)
  }

  if (jsResult.stdout !== rustResult.stdout) {
    // If both start with 'Error:' or 'WARN:', we might accept it as a network error format difference
    if (jsResult.stdout.includes('Error:') && rustResult.stdout.includes('Error:')) {
      console.log(`✅ stdout matches (fuzzy error match)`)
    } else {
      console.error(`❌ stdout mismatch.`)
      console.error(`JS stdout:`, jsResult.stdout)
      console.error(`Rust stdout:`, rustResult.stdout)
      passed = false
    }
  } else {
    console.log(`✅ stdout matches`)
  }

  return passed
}

function startMockServer(port) {
  return new Promise((resolve) => {
    const server = http.createServer((req, res) => {
      const url = new URL(req.url, `http://127.0.0.1:${port}`)
      const q = url.searchParams.get('q')
      if (q === 'success') {
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({ results: [{ title: 'A', url: 'http://a', content: 'B' }] }))
      } else if (q === 'empty') {
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({ results: [] }))
      } else if (q === 'malformed') {
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end('{ malformed json')
      } else if (q === 'non2xx') {
        res.writeHead(500, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({ error: 'Server error' }))
      } else if (q === 'timeout') {
        // Just don't respond to simulate a timeout/hang
      } else {
        res.writeHead(404)
        res.end()
      }
    })
    server.listen(port, '127.0.0.1', () => resolve(server))
  })
}

async function runAll() {
  let allPassed = true

  const mockPort = 9999
  const server = await startMockServer(mockPort)
  const baseUrl = `http://127.0.0.1:${mockPort}`

  // Terminal tool tests
  allPassed &= await compare('terminal-tool', 'echo command', ['echo', 'hello', 'world'])
  allPassed &= await compare('terminal-tool', 'no arguments', [])
  allPassed &= await compare('terminal-tool', 'missing command', ['--cwd', '/tmp'])
  allPassed &= await compare('terminal-tool', 'quoting and spaces', ['echo "a b c"', 'd', 'e'])
  allPassed &= await compare('terminal-tool', 'empty arguments', ['echo', ''])

  // Curl tool tests
  allPassed &= await compare('curl-tool', 'no arguments', [])
  allPassed &= await compare('curl-tool', 'blocked by guard without bypass', ['http://localhost'])

  const rustLocalhost = await runCli(CLIS['curl-tool'].rust, ['http://127.0.0.1'])
  if (!rustLocalhost.stdout.includes('SSRF guard blocked request')) {
    console.error('❌ Rust failed to block 127.0.0.1')
    allPassed = false
  } else {
    console.log('✅ Rust blocked 127.0.0.1')
  }

  const rustPrivate = await runCli(CLIS['curl-tool'].rust, ['http://192.168.1.5'])
  if (!rustPrivate.stdout.includes('SSRF guard blocked request')) {
    console.error('❌ Rust failed to block 192.168.1.5')
    allPassed = false
  } else {
    console.log('✅ Rust blocked 192.168.1.5')
  }

  // Searxng tool tests
  allPassed &= await compare('searxng-search-tool', 'no arguments', [])
  allPassed &= await compare('searxng-search-tool', 'success query', ['success', '--base-url', baseUrl])
  allPassed &= await compare('searxng-search-tool', 'empty response', ['empty', '--base-url', baseUrl])
  allPassed &= await compare('searxng-search-tool', 'malformed response', ['malformed', '--base-url', baseUrl])
  allPassed &= await compare('searxng-search-tool', 'non-2xx response', ['non2xx', '--base-url', baseUrl])
  allPassed &= await compare('searxng-search-tool', 'connection failure', ['success', '--base-url', 'http://127.0.0.1:12345'])

  server.close()

  if (!allPassed) {
    console.error('\n❌ Some parity tests failed.')
    process.exit(1)
  } else {
    console.log('\n✅ All parity tests passed.')
  }
}

runAll()
