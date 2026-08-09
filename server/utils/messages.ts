import { eq, and, asc } from 'drizzle-orm'
import { createParser } from 'eventsource-parser'
import { conversations, messages as messagesTable } from '../database/schema'
import type { UIMessage } from '#shared/types/chat'

async function requireOwnedConversation(userId: string, conversationId: string) {
  const db = useDb()
  const [conv] = await db
    .select()
    .from(conversations)
    .where(and(eq(conversations.id, conversationId), eq(conversations.userId, userId)))
    .limit(1)

  if (!conv) throw notFound('Conversation not found')
  return conv
}

export async function listConversationMessages(userId: string, conversationId: string) {
  const conversation = await requireOwnedConversation(userId, conversationId)
  const db = useDb()

  const msgs = await db
    .select()
    .from(messagesTable)
    .where(eq(messagesTable.conversationId, conversation.id))
    .orderBy(asc(messagesTable.createdAt))

  return {
    id: conversation.id,
    title: conversation.title,
    workspaceId: conversation.workspaceId,
    modelId: conversation.modelId,
    reasoningEffort: conversation.reasoningEffort,
    enabledToolIds: conversation.enabledToolIds,
    approvals: conversation.approvals,
    mode: conversation.mode,
    createdAt: conversation.createdAt.getTime(),
    updatedAt: conversation.updatedAt.getTime(),
    messages: msgs.map(m => ({
      id: m.id,
      role: m.role,
      createdAt: m.createdAt,
      parts: Array.isArray(m.parts) ? m.parts : (typeof m.parts === 'string' ? JSON.parse(m.parts) : m.parts)
    }))
  }
}

/**
 * Non-streaming counterpart to chat.post.ts's send path — same persistence
 * and upstream-call shape, but collects the full reply before returning
 * instead of writing an ai@7 UIMessageChunk stream. Used by the MCP
 * `send_message` tool, which is request/response, not a live SSE client.
 */
export async function sendMessage(userId: string, conversationId: string, text: string) {
  const conv = await requireOwnedConversation(userId, conversationId)
  const db = useDb()

  await db.insert(messagesTable).values({
    conversationId: conv.id,
    role: 'user',
    parts: [{ type: 'text', text }]
  })

  const priorMessages = await db
    .select()
    .from(messagesTable)
    .where(eq(messagesTable.conversationId, conv.id))
    .orderBy(asc(messagesTable.createdAt))

  const mappedMessages = priorMessages.map((m) => {
    const parts = (Array.isArray(m.parts) ? m.parts : (typeof m.parts === 'string' ? JSON.parse(m.parts) : m.parts)) as UIMessage['parts']
    let content = ''
    for (const part of parts) {
      if (part.type === 'text') content += part.text
    }
    return { role: m.role, content }
  })

  const config = useRuntimeConfig()

  const response = await fetch(`${config.routerBaseUrl}/chat/completions`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${config.routerApiKey}`
    },
    body: JSON.stringify({
      model: conv.modelId || 'vx/gemini-3-flash-preview',
      messages: mappedMessages,
      stream: true
    })
  })

  if (!response.ok) {
    throw new Error(`Upstream error: ${response.status} ${response.statusText}`)
  }

  let fullResponseText = ''
  const parser = createParser({
    onEvent: (event) => {
      if (event.data === '[DONE]') return
      try {
        const data = JSON.parse(event.data)
        const delta = data.choices[0]?.delta?.content || ''
        if (delta) fullResponseText += delta
      } catch (err) {
        logger.error('Error parsing SSE', err)
      }
    }
  })

  const reader = response.body?.getReader()
  if (reader) {
    const decoder = new TextDecoder()
    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      parser.feed(decoder.decode(value))
    }
  }

  const [assistantMessage] = await db.insert(messagesTable).values({
    conversationId: conv.id,
    role: 'assistant',
    parts: [{ type: 'text', text: fullResponseText || 'Empty response' }]
  }).returning()

  return {
    id: assistantMessage?.id,
    role: 'assistant',
    text: fullResponseText || 'Empty response'
  }
}
