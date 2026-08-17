#!/usr/bin/env node
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, extname, join, relative, resolve, sep } from 'node:path'

const ROOT = resolve(import.meta.dirname, '..')

const POLICY = Object.freeze({
  sourceRoots: ['app', 'server', 'shared', 'packages'],
  sourceExtensions: new Set(['.ts', '.tsx', '.js', '.mjs', '.vue', '.rs', '.css', '.scss', '.sass', '.less', '.styl', '.stylus']),
  excludedSegments: new Set([
    'node_modules', 'target', '.nuxt', '.output', 'dist', 'coverage', 'vendor', 'generated',
    'migrations', '.agents', '.tmp'
  ]),
  file: { review: 400, hard: 500 },
  folder: { reviewMin: 13, hard: 15 },
  exceptions: {
    files: new Map(),
    folders: new Map([
      ['app/composables', 'Nuxt public use* auto-import entrypoints form one framework-owned API surface; splitting or wrapper re-exports would add indirection without reducing responsibility.']
    ])
  }
})

function assertExceptionPolicy() {
  for (const [kind, entries] of Object.entries(POLICY.exceptions)) {
    for (const [path, reason] of entries) {
      if (!path || path.includes('*') || path.includes('?') || path.endsWith('/') || path === '.') {
        throw new Error(`maintainability: invalid broad ${kind} exception: ${path || '<empty>'}`)
      }
      if (!reason || reason.trim().length < 20) {
        throw new Error(`maintainability: ${kind} exception requires a concrete reason: ${path}`)
      }
    }
  }
}

function isMaintainedSource(absPath, root) {
  const rel = relative(root, absPath)
  if (!rel || rel.startsWith(`..${sep}`) || rel === '..') return false
  const segments = rel.split(sep)
  if (segments.some(segment => POLICY.excludedSegments.has(segment))) return false
  return POLICY.sourceExtensions.has(extname(absPath))
}

function walk(dir, root, files) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const abs = join(dir, entry.name)
    const relSegments = relative(root, abs).split(sep)
    if (relSegments.some(segment => POLICY.excludedSegments.has(segment))) continue
    if (entry.isDirectory()) walk(abs, root, files)
    else if (entry.isFile() && isMaintainedSource(abs, root)) files.push(abs)
  }
}

function physicalLines(path) {
  const text = readFileSync(path, 'utf8')
  if (text.length === 0) return 0
  const lines = text.split(/\r?\n/).length
  return text.endsWith('\n') ? lines - 1 : lines
}

function inspect(root = ROOT) {
  assertExceptionPolicy()
  const files = []
  for (const sourceRoot of POLICY.sourceRoots) {
    const abs = join(root, sourceRoot)
    try {
      walk(abs, root, files)
    } catch (error) {
      if (error?.code !== 'ENOENT') throw error
    }
  }

  const hardFailures = []
  const reviewFindings = []
  const folderCounts = new Map()

  for (const abs of files) {
    const rel = relative(root, abs).split(sep).join('/')
    const lines = physicalLines(abs)
    const folder = dirname(rel).split(sep).join('/')
    folderCounts.set(folder, (folderCounts.get(folder) ?? 0) + 1)

    if (lines > POLICY.file.hard) {
      const reason = POLICY.exceptions.files.get(rel)
      if (!reason) hardFailures.push(`file ${rel}: ${lines} lines > ${POLICY.file.hard}`)
      else reviewFindings.push(`allowed file exception ${rel}: ${lines} lines — ${reason}`)
    } else if (lines > POLICY.file.review) {
      reviewFindings.push(`review file ${rel}: ${lines} lines`)
    }
  }

  for (const [folder, count] of [...folderCounts.entries()].sort()) {
    if (count > POLICY.folder.hard) {
      const reason = POLICY.exceptions.folders.get(folder)
      if (!reason) hardFailures.push(`folder ${folder}: ${count} direct maintained files > ${POLICY.folder.hard}`)
      else reviewFindings.push(`allowed folder exception ${folder}: ${count} files — ${reason}`)
    } else if (count >= POLICY.folder.reviewMin) {
      reviewFindings.push(`review folder ${folder}: ${count} direct maintained files`)
    }
  }

  return { files: files.length, hardFailures, reviewFindings }
}

function selfTest() {
  const fixture = mkdtempSync(join(tmpdir(), 'ai-code-maintainability-'))
  try {
    mkdirSync(join(fixture, 'app', 'feature'), { recursive: true })
    writeFileSync(join(fixture, 'app', 'oversized.ts'), `${'x\n'.repeat(POLICY.file.hard + 1)}`)
    for (let i = 0; i < POLICY.folder.hard + 1; i++) {
      writeFileSync(join(fixture, 'app', 'feature', `f${i}.ts`), 'export {}\n')
    }
    const result = inspect(fixture)
    const hasFile = result.hardFailures.some(item => item.includes('oversized.ts'))
    const hasFolder = result.hardFailures.some(item => item.includes('app/feature'))
    if (!hasFile || !hasFolder) {
      throw new Error(`maintainability self-test failed: ${result.hardFailures.join('; ')}`)
    }
    console.log('maintainability self-test: PASS — oversized-file and overfull-folder fixtures rejected')
  } finally {
    rmSync(fixture, { recursive: true, force: true })
  }
}

if (process.argv.includes('--self-test')) {
  selfTest()
  process.exit(0)
}

const result = inspect()
for (const finding of result.reviewFindings) console.log(`maintainability: ${finding}`)
if (result.hardFailures.length > 0) {
  for (const failure of result.hardFailures) console.error(`maintainability: ERROR ${failure}`)
  process.exit(1)
}
console.log(`maintainability: PASS — ${result.files} maintained source files checked; no unexplained hard violations`)
