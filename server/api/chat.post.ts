import { messages as messagesTable, conversations } from '../database/schema'
import { eq, and } from 'drizzle-orm'
import { createParser } from 'eventsource-parser'
import { createUIMessageStream, createUIMessageStreamResponse } from 'ai'
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

  // Builds a real ai@7 UIMessageChunk stream — useChat()'s DefaultChatTransport
  // parses Server-Sent Events against uiMessageChunkSchema, not the legacy
  // `0:"..."` data-stream-protocol lines this used to emit. See
  // .agents/memories/ai-sdk-native-features.md: use the SDK's own stream
  // helpers rather than hand-rolling the wire format.
  const stream = createUIMessageStream({
    execute: async ({ writer }) => {
      const abortController = new AbortController()
      event.node.req.on('close', () => {
        abortController.abort()
      })

      const response = await fetch(`${config.routerBaseUrl}/chat/completions`, {
        method: 'POST',
        signal: abortController.signal,
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
      const textId = crypto.randomUUID()
      writer.write({ type: 'text-start', id: textId })

      const parser = createParser({
        onEvent: (event) => {
          if (event.data === '[DONE]') {
            return
          }
          try {
            const data = JSON.parse(event.data)
            const delta = data.choices[0]?.delta?.content || ''
            if (delta) {
              fullResponseText += delta
              writer.write({ type: 'text-delta', id: textId, delta })
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

      writer.write({ type: 'text-end', id: textId })

      await db.insert(messagesTable).values({
        conversationId: conv.id,
        role: 'assistant',
        parts: [{ type: 'text', text: fullResponseText || 'Empty response' }]
      })
    },
    onError: (err) => {
      console.error('[chat stream]', err)
      return 'Something went wrong while generating a response.'
    }
  })

  return createUIMessageStreamResponse({ stream })
})
