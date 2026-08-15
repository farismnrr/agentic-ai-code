import fs from 'node:fs/promises'

export default defineEventHandler(async (event) => {
  await requireUserSession(event)

  const query = getQuery(event)
  const relativePath = (query.path as string) || ''

  const telemetry = event.context.application.observability.request

  return telemetry.withSpan('workspace.fs.browse', {}, async () => {
    const resolvedPath = await event.context.application.filesystem.resolveWorkspacePath(relativePath)

    try {
      const entries = await fs.readdir(resolvedPath, { withFileTypes: true })
      const directories = entries
        .filter(entry => entry.isDirectory())
        .map(entry => ({
          name: entry.name,
          path: relativePath ? `${relativePath}/${entry.name}`.replace(/^\/+/, '') : entry.name
        }))

      return {
        root: useRuntimeConfig().workspacesRoot,
        path: relativePath,
        entries: directories
      }
    } catch (error: unknown) {
      if (error && typeof error === 'object' && 'code' in error && error.code === 'ENOENT') {
        throw createError({ statusCode: 404, statusMessage: 'Directory not found' })
      }
      throw error
    }
  })
})
