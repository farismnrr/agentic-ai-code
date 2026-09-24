import { readFile, readdir, stat } from 'node:fs/promises'
import { extname, join, relative, resolve } from 'node:path'

const target = process.argv[2]
if (!target) throw new Error('usage: node scripts/guardrail.mjs <service-directory>')

const root = resolve(target)
const sourceExtensions = new Set(['.rs', '.ts', '.svelte', '.css', '.mjs'])
const ignoredDirectories = new Set(['node_modules', 'target', 'dist', '.git'])
const limits = {
  '.rs': 220,
  '.ts': 180,
  '.svelte': 180,
  '.css': 180,
  '.mjs': 240
}
const maxFilesPerDirectory = 12
const violations = []

async function walk(directory) {
  const entries = await readdir(directory, { withFileTypes: true })
  const sourceFiles = entries.filter(entry =>
    entry.isFile() && sourceExtensions.has(extname(entry.name))
  )

  if (sourceFiles.length > maxFilesPerDirectory) {
    violations.push(
      `${relative(root, directory) || '.'}: ${sourceFiles.length} source files; max is ${maxFilesPerDirectory}. Split by responsibility.`
    )
  }

  for (const entry of entries) {
    if (entry.isDirectory()) {
      if (!ignoredDirectories.has(entry.name)) await walk(join(directory, entry.name))
      continue
    }

    if (!entry.isFile()) continue

    const extension = extname(entry.name)
    if (!sourceExtensions.has(extension)) continue

    const path = join(directory, entry.name)
    const content = await readFile(path, 'utf8')
    checkFileBudget(path, extension, content)
    checkArchitectureBoundary(path, content)
  }
}

function checkFileBudget(path, extension, content) {
  const lineCount = content.split('\n').length
  const limit = limits[extension]

  if (limit && lineCount > limit) {
    violations.push(
      `${relative(root, path)}: ${lineCount} lines; max is ${limit}. Split by responsibility.`
    )
  }
}

function checkArchitectureBoundary(path, content) {
  const normalized = relative(root, path).replaceAll('\\', '/')
  const isInnerLayer =
    normalized.startsWith('src/domain/')
    || normalized.startsWith('src/application/')

  if (!isInnerLayer) return

  for (const dependency of ['axum', 'reqwest', 'tokio', 'tower', 'sqlx', 'diesel', 'sea_orm']) {
    const matcher = new RegExp(
      `(^|[^A-Za-z0-9_])${dependency}(::|[^A-Za-z0-9_])`,
      'm'
    )

    if (matcher.test(content)) {
      violations.push(
        `${normalized}: inner architecture layer must not depend on ${dependency}.`
      )
    }
  }
}

await stat(root)
await walk(root)

if (violations.length) {
  console.error('Guardrail failed:\n')
  for (const violation of violations) console.error(`- ${violation}`)
  process.exit(1)
}

console.log('Guardrail passed.')
