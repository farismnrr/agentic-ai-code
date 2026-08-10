import fs from 'node:fs/promises'
import path from 'node:path'

function isWithinRoot(candidate: string, root: string): boolean {
  return candidate === root || candidate.startsWith(root + path.sep)
}

/**
 * Resolves a target path against a root workspace directory and ensures
 * it does not traverse outside the root (including symlinks).
 */
export async function resolveScopedPath(targetPath: string, rootDir: string): Promise<string> {
  const absoluteRoot = path.resolve(rootDir)
  const cleanTarget = targetPath.replace(/^[/\\]+/, '')
  const resolvedPath = path.isAbsolute(targetPath)
    ? path.resolve(targetPath)
    : path.resolve(absoluteRoot, cleanTarget)

  if (!isWithinRoot(resolvedPath, absoluteRoot)) {
    throw new Error(`Path traversal blocked: ${targetPath} is outside workspace ${absoluteRoot}`)
  }

  try {
    const [realRoot, realPath] = await Promise.all([
      fs.realpath(absoluteRoot),
      fs.realpath(resolvedPath)
    ])
    if (!isWithinRoot(realPath, realRoot)) {
      throw new Error(`Symlink traversal blocked: ${targetPath} resolves outside workspace ${absoluteRoot}`)
    }
  } catch (err: unknown) {
    if (err && typeof err === 'object' && 'code' in err && (err as { code: string }).code === 'ENOENT') {
      return resolvedPath
    }
    throw err
  }

  return resolvedPath
}
