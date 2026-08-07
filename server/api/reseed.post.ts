import { defineEventHandler } from 'h3'
import { conversations, messages, userSettings, mcpServers, workspaces } from '../database/schema'
import { eq } from 'drizzle-orm'
import { seedConversations } from '#shared/utils/fixtures/conversations'
import { defaultModelId } from '#shared/utils/fixtures/models'
import { mcpServers as seedMcpServers } from '#shared/utils/fixtures/mcp-servers'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const userId = session.user.id
  const db = useDb()

  // Clear existing data
  await db.delete(mcpServers).where(eq(mcpServers.userId, userId))
  await db.delete(userSettings).where(eq(userSettings.userId, userId))
  await db.delete(conversations).where(eq(conversations.userId, userId))

  // Reseed settings
  await db.insert(userSettings).values({
    userId,
    language: 'en',
    streaming: true,
    sendOnEnter: true,
    defaultModelId: defaultModelId,
    temperature: 0.7,
    systemPrompt: '',
    displayName: session.user?.name || session.user?.email || 'User',
    email: session.user?.email || ''
  })

  // Reseed MCP servers
  const mcpData = seedMcpServers.map(s => ({
    ...s,
    id: `${s.id}-${userId}`,
    userId
  }))
  if (mcpData.length) {
    await db.insert(mcpServers).values(mcpData)
  }

  let [w] = await db.select().from(workspaces).where(eq(workspaces.userId, userId)).limit(1)
  if (!w) {
    const [inserted] = await db.insert(workspaces).values({ userId, name: 'Personal' }).returning()
    w = inserted!
  }

  // Reseed conversations and messages
  for (const conv of seedConversations) {
    const [c] = await db.insert(conversations).values({
      userId,
      workspaceId: w.id,
      title: conv.title,
      modelId: conv.modelId,
      enabledToolIds: conv.enabledToolIds || [],
      approvals: conv.approvals ?? {},
      createdAt: new Date(conv.createdAt),
      updatedAt: new Date(conv.updatedAt)
    }).returning()

    if (!c) continue

    if (conv.messages && conv.messages.length) {
      const msgs = conv.messages.map(m => ({
        conversationId: c.id,
        role: m.role,
        parts: m.parts || [],
        createdAt: new Date()
      }))
      await db.insert(messages).values(msgs)
    }
  }

  return { ok: true }
})
