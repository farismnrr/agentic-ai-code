import { mcpServers } from '../../database/schema'
import * as v from 'valibot'

const createSchema = v.object({
  name: v.string(),
  description: v.optional(v.string(), ''),
  transport: v.string(),
  url: v.optional(v.string()),
  command: v.optional(v.string())
})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const db = useDb()

  const result = v.safeParse(createSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  const id = body.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || `server-${Date.now().toString(36)}`

  let server
  try {
    [server] = await db
      .insert(mcpServers)
      .values({
        id: `${id}-${session.user.id}`, // To ensure uniqueness per user if necessary, or just rely on UUID/timestamp. Wait, id is a string and primary key. Let's make it unique across all by appending timestamp
        userId: session.user.id,
        name: body.name,
        description: body.description,
        transport: body.transport,
        url: body.url,
        command: body.command,
        status: 'connected',
        enabled: true,
        tools: []
      })
      .returning()
  } catch (err) {
    if (isUniqueViolation(err)) throw conflict('Server ID already exists')
    throw err
  }

  if (!server) {
    throw internal('Failed to create server')
  }

  return {
    id: server.id,
    name: server.name,
    description: server.description,
    transport: server.transport,
    url: server.url,
    command: server.command,
    status: server.status,
    enabled: server.enabled,
    tools: []
  }
})
