import { messages as messagesTable, conversations } from '../database/schema'
import { eq, and } from 'drizzle-orm'
import { createParser } from 'eventsource-parser'
import type { UIMessage } from '#shared/types/chat'

function mapMessages(messages: UIMessage[]) {
  return messages.map((msg) => {
    let text = ''
    if (msg.parts) {
      for (const part of msg.parts) {
        if (part.type === 'text') {
          text += part.text
        }
      }
    }
    return {
      role: msg.role,
      content: text
    }
  })
}

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const { messages, id: conversationId } = await readBody(event)

  if (!conversationId) {
    throw badRequest('Missing conversationId')
  }

  const db = useDb()

  const [conv] = await db
    .select()
    .from(conversations)
    .where(and(eq(conversations.id, conversationId), eq(conversations.userId, session.user.id)))
    .limit(1)

  if (!conv) {
    throw notFound('Conversation not found')
  }

  const lastMsg = messages[messages.length - 1]
  if (lastMsg && lastMsg.role === 'user') {
    await db.insert(messagesTable).values({
      conversationId: conv.id,
      role: 'user',
      parts: lastMsg.parts
    })
  }

  const mappedMessages = mapMessages(messages)
  const config = useRuntimeConfig()

  setHeader(event, 'content-type', 'text/plain; charset=utf-8')

  return new ReadableStream({
    async start(controller) {
      try {
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

        const parser = createParser((event) => {
          if (event.type === 'event') {
            if (event.data === '[DONE]') {
              return
            }
            try {
              const data = JSON.parse(event.data)
              const delta = data.choices[0]?.delta?.content || ''
              if (delta) {
                fullResponseText += delta
                controller.enqueue(`0:${JSON.stringify(delta)}\n`)
              }
            } catch (err) {
              console.error('Error parsing SSE', err)
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

        await db.insert(messagesTable).values({
          conversationId: conv.id,
          role: 'assistant',
          parts: [{ type: 'text', text: fullResponseText || 'Mock response without text' }]
        })

        controller.close()
      } catch (err) {
        console.error('[chat stream]', err)
        controller.enqueue(`3:${JSON.stringify('Something went wrong while generating a response.')}\n`)
        controller.close()
      }
    }
  })
})
