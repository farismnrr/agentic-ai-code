import path from 'node:path'

/**
 * Resolves a requested relative path against the configured NUXT_WORKSPACES_ROOT.
 * Ensures the resolved path stays within the root directory (prevents directory traversal).
 *
 * @param relativePath - The requested relative path
 * @returns The resolved absolute path, or throws an error if invalid/out of bounds
 */
export function resolveWorkspacePath(relativePath: string): string {
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
  if (!resolvedPath.startsWith(absoluteRoot)) {
    throw createError({ statusCode: 403, statusMessage: 'Path traversal detected' })
  }

  return resolvedPath
}
