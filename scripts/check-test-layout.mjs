#!/usr/bin/env node
import fs from 'node:fs'
import { execFileSync } from 'node:child_process'
import path from 'node:path'

const ROOT = path.resolve(import.meta.dirname, '..')
const SOURCE_EXTENSIONS = new Set(['.js', '.mjs', '.ts', '.tsx', '.vue', '.rs'])
const requestedScope = process.argv[2] ?? 'all'
if (!new Set(['all', 'nuxt', 'rust']).has(requestedScope)) {
  console.error(`Test layout guard: invalid scope ${requestedScope}; expected all, nuxt, or rust`)
  process.exit(2)
}

const SKIP_DIRS = new Set(['.git', '.nuxt', '.output', 'node_modules', 'target', 'dist', 'generated', 'migrations', '.agents', 'workspaces', '.worktrees', '.pnpm-store'])

function walk(dir, files = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (SKIP_DIRS.has(entry.name)) continue
    const file = path.join(dir, entry.name)
    if (entry.isDirectory()) walk(file, files)
    else if (SOURCE_EXTENSIONS.has(path.extname(file))) files.push(file)
  }
  return files
}

function relative(file) {
  return path.relative(ROOT, file).split(path.sep).join('/')
}

function isApprovedTestLocation(file) {
  const rel = relative(file)
  const extension = path.extname(file)
  if (extension === '.rs') return rel.startsWith('packages/rust-tools/tests/')
  return rel.startsWith('test/')
}

function belongsToScope(file) {
  if (requestedScope === 'all') return true
  return requestedScope === 'rust'
    ? path.extname(file) === '.rs'
    : path.extname(file) !== '.rs'
}

function lineNumber(source, index) {
  return source.slice(0, index).split('\n').length
}

function sourceWithoutComments(source) {
  return source.replaceAll(/\/\*[\s\S]*?\*\//g, comment => comment.replaceAll(/[^\n]/g, ' ')).replaceAll(/\/\/.*$/gm, comment => comment.replaceAll(/[^\n]/g, ' '))
}

function testFileName(file) {
  const extension = path.extname(file)
  const basename = path.basename(file, extension)
  return /(?:\.test|\.spec|[_-]tests?)$/i.test(basename) || /^(?:test|tests|spec|specs)$/i.test(basename)
}

function rustFailures(file, source) {
  const failures = []
  for (const pattern of [
    /#\s*\[\s*(?:(?:[A-Za-z_]\w*::)*(?:test|rstest|test_case)(?:\s*\([^]]*\))?|(?:cfg|cfg_attr)\s*\([^]]*\btest\b[^]]*\))\s*\]/gs,
    /\bmod\s+tests?\s*\{/g
  ]) {
    for (const match of source.matchAll(pattern)) {
      failures.push(`${relative(file)}:${lineNumber(source, match.index)}: unit tests must live in a dedicated test folder`)
    }
  }
  return failures
}

function javascriptFailures(file, source) {
  const code = sourceWithoutComments(source)
  const frameworkImport = /from\s+['"](?:vitest|jest|@jest\/globals|mocha|node:test|bun:test)['"]|require\(\s*['"](?:vitest|jest|mocha|node:test|bun:test)['"]\s*\)/.test(source)
  const patterns = [
    /(?<![\w.$])(?:describe|it|beforeEach|afterEach)\s*\(/g,
    ...(frameworkImport ? [/(?<![\w.$])(?:test|expect)\s*\(/g] : [])
  ]
  return patterns.flatMap(pattern => [...code.matchAll(pattern)].map(match => (
    `${relative(file)}:${lineNumber(source, match.index)}: unit tests must live in a dedicated test folder`
  )))
}

function check(root = ROOT) {
  const files = walk(root).filter(belongsToScope)
  return files.flatMap((file) => {
    if (isApprovedTestLocation(file)) return []
    if (testFileName(file)) return [`${relative(file)}: web tests belong under test/; Rust tests belong under packages/rust-tools/tests/`]
    const source = fs.readFileSync(file, 'utf8')
    if (path.extname(file) === '.rs') return rustFailures(file, source)
    if (['.js', '.mjs', '.ts', '.tsx', '.vue'].includes(path.extname(file))) return javascriptFailures(file, source)
    return []
  })
}

function addedUnitTestFailures() {
  const range = process.env.AI_CODE_GUARD_BASE_SHA && process.env.AI_CODE_GUARD_HEAD_SHA
    ? [process.env.AI_CODE_GUARD_BASE_SHA, process.env.AI_CODE_GUARD_HEAD_SHA]
    : ['HEAD']
  const output = execFileSync('git', ['diff', '--name-only', '--diff-filter=A', ...range], { cwd: ROOT, encoding: 'utf8' })
  return output.split('\n').filter(Boolean).flatMap((file) => {
    const lower = file.toLowerCase()
    const base = lower.split('/').pop() ?? lower
    const isUnit = lower.startsWith('test/unit/') || /\.unit\.(test|spec)\.(ts|tsx|js|jsx|mjs)$/.test(base)
    return isUnit ? [`${file}: permanent isolated unit tests are forbidden; use a boundary-level integration/E2E/contract/smoke/regression test or delete the temporary test before commit`] : []
  })
}

const failures = [...check(), ...addedUnitTestFailures()]
if (failures.length) {
  console.error(`Test layout guard failed:\n${failures.map(failure => `- ${failure}`).join('\n')}`)
  process.exit(1)
}
console.log(`Test layout guard passed (${requestedScope}): web tests are under test/ and Rust tests are under packages/rust-tools/tests/`)
