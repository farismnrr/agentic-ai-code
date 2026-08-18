import { readFileSync, realpathSync, statSync } from 'node:fs'
import { isAbsolute, join, relative, sep } from 'node:path'

function isContained(root: string, candidate: string) {
  const rel = relative(root, candidate)
  return rel.length > 0 && rel !== '..' && !rel.startsWith(`..${sep}`) && !isAbsolute(rel)
}

function isMissing(error: unknown) {
  return typeof error === 'object' && error !== null && 'code' in error && (error as { code?: unknown }).code === 'ENOENT'
}

/**
 * Read one runtime instruction file only after both its approved root and final
 * canonical file are proven to remain inside the canonical application root.
 * Symlinked roots/files that escape the image/worktree fail closed.
 */
export function readRuntimeInstruction(
  applicationRoot: string,
  approvedRoot: string[],
  fileParts: string[],
  options: { optional?: boolean } = {}
): { canonical: string, text: string } | undefined {
  let canonicalApplicationRoot: string
  let canonicalApprovedRoot: string
  let canonicalFile: string
  try {
    canonicalApplicationRoot = realpathSync(applicationRoot)
    canonicalApprovedRoot = realpathSync(join(canonicalApplicationRoot, ...approvedRoot))
    if (!isContained(canonicalApplicationRoot, canonicalApprovedRoot)) throw new Error('runtime instruction root escapes application root')
    canonicalFile = realpathSync(join(canonicalApprovedRoot, ...fileParts))
  } catch (error) {
    if (options.optional && isMissing(error)) return undefined
    throw error
  }
  if (!isContained(canonicalApprovedRoot, canonicalFile)) throw new Error('runtime instruction file escapes approved root')
  if (!statSync(canonicalFile).isFile()) throw new Error('runtime instruction target is not a regular file')
  return { canonical: canonicalFile, text: readFileSync(canonicalFile, 'utf8') }
}
