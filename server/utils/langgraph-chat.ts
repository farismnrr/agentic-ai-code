import { createAgent } from 'langchain'
import { getLanggraphModel } from './langgraph-model'
import { langgraphTools } from './langgraph-tools'
import type { UIMessage } from '#shared/types/chat'
import { createUIMessageStream } from 'ai'
import { HumanMessage, AIMessage, SystemMessage } from '@langchain/core/messages'

function convertToLangchainMessages(uiMessages: UIMessage[]) {
  return uiMessages.map((m) => {
    let content = ''
    if (m.parts) {
      for (const part of m.parts) {
        if (part.type === 'text') content += part.text
      }
    }
    if (m.role === 'user') return new HumanMessage(content)
    if (m.role === 'system') return new SystemMessage(content)
    return new AIMessage(content)
  })
}

export function runLanggraphChat(uiMessages: UIMessage[], modelId: string, onEnd: (parts: UIMessage['parts']) => Promise<void>) {
  const model = getLanggraphModel(modelId)
  const agent = createAgent({ model, tools: langgraphTools })

  return createUIMessageStream({
    async execute({ writer }) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const parts: any[] = []
      let currentText = ''
      let textIndex = 0

      try {
        const inputMessages = convertToLangchainMessages(uiMessages)
        const stream = agent.streamEvents(
          { messages: inputMessages },
          { version: 'v2' }
        )

        for await (const event of stream) {
          if (event.event === 'on_chat_model_stream') {
            const chunk = event.data?.chunk?.content
            if (chunk) {
              if (!currentText) {
                textIndex++
                writer.write({ type: 'text-start', id: `text-${textIndex}` })
              }
              writer.write({
                type: 'text-delta',
                id: `text-${textIndex}`,
                delta: chunk
              })
              currentText += chunk
            }
          } else if (event.event === 'on_tool_start') {
            if (currentText) {
              writer.write({ type: 'text-end', id: `text-${textIndex}` })
              parts.push({ type: 'text', text: currentText })
              currentText = ''
            }
            parts.push({
              type: 'dynamic-tool',
              toolCallId: event.run_id,
              toolName: event.name,
              state: 'input-available',
              input: event.data?.input || {}
            })
            writer.write({
              type: 'tool-input-available',
              toolCallId: event.run_id,
              toolName: event.name,
              input: event.data?.input || {},
              dynamic: true
            })
          } else if (event.event === 'on_tool_end') {
            const invocation = parts.find(p => p.type === 'dynamic-tool' && p.toolCallId === event.run_id)
            if (invocation) {
              invocation.state = 'output-available'
              invocation.output = event.data?.output || ''
            }
            writer.write({
              type: 'tool-output-available',
              toolCallId: event.run_id,
              output: event.data?.output || '',
              dynamic: true
            })
          }
        }

        if (currentText) {
          writer.write({ type: 'text-end', id: `text-${textIndex}` })
          parts.push({ type: 'text', text: currentText })
        }

        try {
          await onEnd(parts)
        } catch (err) {
          console.error('[langgraph onEnd] failed', err)
        }
      } catch (e: unknown) {
        if (currentText) {
          writer.write({ type: 'text-end', id: `text-${textIndex}` })
          parts.push({ type: 'text', text: currentText })
        }
        const errorText = (e as Error).message
        for (const part of parts) {
          if (part.type === 'dynamic-tool' && part.state === 'input-available') {
            part.state = 'output-error'
            part.errorText = errorText
          }
        }
        writer.write({
          type: 'error',
          errorText
        })
        if (parts.length > 0) {
          try {
            await onEnd(parts)
          } catch (err) {
            console.error('[langgraph onEnd] failed', err)
          }
        }
      }
    }
  })
}
