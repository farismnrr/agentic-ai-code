import { eq, and } from 'drizzle-orm'
import { mcpServers } from '../../database/schema'
import * as v from 'valibot'

const updateSchema = v.object({
  name: v.optional(v.string()),
  description: v.optional(v.string()),
  transport: v.optional(v.string()),
  url: v.optional(v.string()),
  command: v.optional(v.string()),
  status: v.optional(v.string()),
  enabled: v.optional(v.boolean()),
  tools: v.optional(v.array(v.any()))
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw createError({ statusCode: 400, message: 'Missing server ID' })

  const body = await readValidatedBody(event, data => v.parse(updateSchema, data))
  const db = useDb()

  const [updated] = await db
    .update(mcpServers)
    .set({
      ...(body.name !== undefined && { name: body.name }),
      ...(body.description !== undefined && { description: body.description }),
      ...(body.transport !== undefined && { transport: body.transport }),
      ...(body.url !== undefined && { url: body.url }),
      ...(body.status !== undefined && { status: body.status }),
      ...(body.enabled !== undefined && { enabled: body.enabled }),
      ...(body.tools !== undefined && { tools: body.tools }),
      ...(body.command !== undefined && { command: body.command }),
      updatedAt: new Date()
    })
    .where(and(eq(mcpServers.id, id), eq(mcpServers.userId, session.user.id)))
    .returning()

  if (!updated) {
    throw createError({ statusCode: 404, message: 'Server not found' })
  }

  return {
    id: updated.id,
    name: updated.name,
    description: updated.description,
    transport: updated.transport,
    url: updated.url,
    command: updated.command,
    status: updated.status,
    enabled: updated.enabled,
    tools: Array.isArray(updated.tools) ? updated.tools : (typeof updated.tools === 'string' ? JSON.parse(updated.tools) : updated.tools)
  }
})
