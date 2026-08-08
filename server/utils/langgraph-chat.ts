import { createReactAgent } from '@langchain/langgraph/prebuilt'
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
  const agent = createReactAgent({ llm: model, tools: langgraphTools })

  return createUIMessageStream({
    async execute({ writer }) {
      try {
        const inputMessages = convertToLangchainMessages(uiMessages)
        const stream = await agent.streamEvents(
          { messages: inputMessages },
          { version: 'v2' }
        )

        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const parts: any[] = []
        let currentText = ''

        for await (const event of stream) {
          if (event.event === 'on_chat_model_stream') {
            const chunk = event.data?.chunk?.content
            if (chunk) {
              writer.write({
                type: 'text-delta',
                id: 'langgraph-text',
                delta: chunk
              })
              currentText += chunk
            }
          } else if (event.event === 'on_tool_start') {
            if (currentText) {
              parts.push({ type: 'text', text: currentText })
              currentText = ''
            }
            parts.push({
              type: 'tool-invocation',
              toolInvocation: {
                state: 'call',
                toolCallId: event.run_id,
                toolName: event.name,
                args: event.data?.input || {}
              }
            })
            writer.write({
              type: 'tool-input-available',
              toolCallId: event.run_id,
              toolName: event.name,
              input: event.data?.input || {}
            })
          } else if (event.event === 'on_tool_end') {
            const invocation = parts.find(p => p.type === 'tool-invocation' && p.toolInvocation.toolCallId === event.run_id)
            if (invocation) {
              invocation.toolInvocation.state = 'result'
              invocation.toolInvocation.result = event.data?.output || ''
            }
            writer.write({
              type: 'tool-output-available',
              toolCallId: event.run_id,
              output: event.data?.output || ''
            })
          }
        }

        if (currentText) {
          parts.push({ type: 'text', text: currentText })
        }

        try {
          await onEnd(parts)
        } catch (err) {
          console.error('[langgraph onEnd] failed', err)
        }
      } catch (e: unknown) {
        writer.write({
          type: 'error',
          errorText: (e as Error).message
        })
      }
    }
  })
}
