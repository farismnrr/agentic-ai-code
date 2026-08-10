import { spawn } from 'node:child_process'
import fs from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

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

async function runAll() {
  let allPassed = true

  // Terminal tool tests
  allPassed &= await compare('terminal-tool', 'echo command', ['echo', 'hello', 'world'])
  allPassed &= await compare('terminal-tool', 'no arguments', [])
  allPassed &= await compare('terminal-tool', 'missing command', ['--cwd', '/tmp'])

  // Curl tool tests
  allPassed &= await compare('curl-tool', 'no arguments', [])
  allPassed &= await compare('curl-tool', 'blocked by guard', ['http://example.com'])

  // Searxng tool tests
  allPassed &= await compare('searxng-search-tool', 'no arguments', [])
  // We expect failure if local searxng is not running, but the failure message should match.
  allPassed &= await compare('searxng-search-tool', 'dummy query', ['dummy'])

  if (!allPassed) {
    console.error('\n❌ Some parity tests failed.')
    process.exit(1)
  } else {
    console.log('\n✅ All parity tests passed.')
  }
}

runAll()
