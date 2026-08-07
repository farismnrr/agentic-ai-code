import { pickScenario } from '#shared/utils/fixtures/replies'
import { messages as messagesTable, conversations } from '../database/schema'
import { eq, and } from 'drizzle-orm'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const { messages, id: conversationId } = await readBody(event)

  if (!conversationId) {
    throw createError({ statusCode: 400, message: 'Missing conversationId' })
  }

  const db = useDb()

  const [conv] = await db
    .select()
    .from(conversations)
    .where(and(eq(conversations.id, conversationId), eq(conversations.userId, session.user.id)))
    .limit(1)

  if (!conv) {
    throw createError({ statusCode: 404, message: 'Conversation not found' })
  }

  const lastMsg = messages[messages.length - 1]
  if (lastMsg && lastMsg.role === 'user') {
    await db.insert(messagesTable).values({
      conversationId: conv.id,
      role: 'user',
      parts: lastMsg.parts
    })
  }

  const prompt = lastMsg?.content ?? ''
  const scenario = pickScenario(prompt)

  setHeader(event, 'content-type', 'text/plain; charset=utf-8')

  return new ReadableStream({
    async start(controller) {
      const chunks = scenario.build({ enabledToolIds: conv.enabledToolIds || [] })
      let fullResponseText = ''

      for (const chunk of chunks) {
        if (chunk.type === 'text-delta') {
          controller.enqueue(`0:${JSON.stringify(chunk.delta)}\n`)
          fullResponseText += chunk.delta
        } else if (chunk.type === 'reasoning-delta') {
          // Send reasoning as normal text for simplicity
          controller.enqueue(`0:${JSON.stringify(chunk.delta)}\n`)
          fullResponseText += chunk.delta
        }

        await new Promise(resolve => setTimeout(resolve, 22))
      }

      await db.insert(messagesTable).values({
        conversationId: conv.id,
        role: 'assistant',
        parts: [{ type: 'text', text: fullResponseText || 'Mock response without text' }]
      })

      controller.close()
    }
  })
})
