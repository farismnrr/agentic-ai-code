import fs from 'node:fs/promises'
import path from 'node:path'

function isWithinRoot(candidate: string, root: string): boolean {
  return candidate === root || candidate.startsWith(root + path.sep)
}

/**
 * Resolves a requested relative path against the configured NUXT_WORKSPACES_ROOT.
 * Ensures the resolved path stays within the root directory (prevents directory traversal).
 *
 * Also resolves symlinks and re-checks the boundary against the *real* path — a
 * string-prefix check alone doesn't catch a symlink inside the root that points
 * outside it, and `fs.readdir`/`fs.stat` both follow symlinks by default.
 *
 * @param relativePath - The requested relative path
 * @returns The resolved absolute path, or throws an error if invalid/out of bounds
 */
export async function resolveWorkspacePath(relativePath: string): Promise<string> {
  const config = useRuntimeConfig()
  const root = config.workspacesRoot

  if (!root) {
    throw createError({ statusCode: 500, statusMessage: 'NUXT_WORKSPACES_ROOT is not configured' })
  }

  // Resolve the absolute root
  const absoluteRoot = path.resolve(root)

  // Resolve the requested path against the root
  // We use `path.join` with `/` as the base, so that leading slashes don't reset to the filesystem root.
  // We strip leading slashes to treat it as purely relative to our workspace root.
  const cleanRelative = relativePath.replace(/^[/\\]+/, '')
  const resolvedPath = path.resolve(absoluteRoot, cleanRelative)

  // Ensure the resolved path still starts with the absolute root (traversal guard)
  // Must check with path.sep to prevent matching sibling directories that share a prefix
  // e.g. /root-dir and /root-dir-sibling
  if (!isWithinRoot(resolvedPath, absoluteRoot)) {
    throw createError({ statusCode: 403, statusMessage: 'Path traversal detected' })
  }

  // Symlink guard: if resolvedPath (or an ancestor of it) is a symlink pointing
  // outside the root, the string check above still passes but the real target
  // doesn't. If the path doesn't exist yet, there's nothing to resolve past —
  // the downstream fs.readdir/fs.stat call will fail on it on its own.
  try {
    const [realRoot, realPath] = await Promise.all([
      fs.realpath(absoluteRoot),
      fs.realpath(resolvedPath)
    ])
    if (!isWithinRoot(realPath, realRoot)) {
      throw createError({ statusCode: 403, statusMessage: 'Path traversal detected' })
    }
  } catch (err) {
    if (err && typeof err === 'object' && 'code' in err && err.code === 'ENOENT') {
      return resolvedPath
    }
    throw err
  }

  return resolvedPath
}
