import { createAgent } from 'langchain'
import { getLanggraphModel } from './langgraph-model'
import { buildLanggraphTools } from './langgraph-tools'
import { createSearxngSearchTool } from '@ai-code/searxng-search-tool'
import type { UIMessage } from '#shared/types/chat'
import { createUIMessageStream } from 'ai'
import { HumanMessage, AIMessage, SystemMessage } from '@langchain/core/messages'

function extractForcedSearch(uiMessages: UIMessage[]) {
  if (uiMessages.length === 0) return { forced: false }
  const lastMessage = uiMessages[uiMessages.length - 1]
  if (!lastMessage || lastMessage.role !== 'user') return { forced: false }

  let content = ''
  if (lastMessage.parts) {
    for (const part of lastMessage.parts) {
      if (part.type === 'text') content += part.text
    }
  }
  const match = content.trim().match(/^@search[:,]?\s+(.+)/is)
  if (match) {
    return { forced: true, cleanedText: match[1] }
  }
  return { forced: false }
}

function convertToLangchainMessages(uiMessages: UIMessage[], cleanedLastText?: string) {
  return uiMessages.map((m, index) => {
    let content = ''
    if (index === uiMessages.length - 1 && cleanedLastText !== undefined) {
      content = cleanedLastText
    } else if (m.parts) {
      for (const part of m.parts) {
        if (part.type === 'text') content += part.text
      }
    }
    if (m.role === 'user') return new HumanMessage(content)
    if (m.role === 'system') return new SystemMessage(content)
    return new AIMessage(content)
  })
}

export function runLanggraphChat(uiMessages: UIMessage[], modelId: string, workspacePath: string | undefined, onEnd: (parts: UIMessage['parts']) => Promise<void>) {
  const { forced, cleanedText } = extractForcedSearch(uiMessages)

  return createUIMessageStream({
    async execute({ writer }) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const parts: any[] = []
      let currentText = ''
      let textIndex = 0

      try {
        const baseModel = getLanggraphModel(modelId)

        if (forced && cleanedText !== undefined) {
          // @search guarantees the tool actually runs — rather than asking
          // the model to decide (via provider-level tool_choice forcing,
          // which turned out unreliable against the installed
          // langchain/@langchain/openai versions: ChatOpenAI.withConfig()
          // doesn't survive createAgent()'s own internal re-binding the way
          // its own source suggests it should — see
          // .agents/memories/019-search-forced-tool-choice-unreliable.md),
          // the tool is called directly here, and the writer chunks that
          // normally come from LangGraph's on_tool_start/on_tool_end are
          // written by hand in the exact same shape. The model is then
          // asked to summarize the real results — it never gets a chance
          // to skip searching, because searching already happened.
          const toolCallId = `search-${Date.now()}`
          parts.push({
            type: 'dynamic-tool',
            toolCallId,
            toolName: 'searxng_search',
            state: 'input-available',
            input: { query: cleanedText }
          })
          writer.write({
            type: 'tool-input-available',
            toolCallId,
            toolName: 'searxng_search',
            input: { query: cleanedText },
            dynamic: true
          })

          let searchResultText: string
          try {
            const searxngSearchTool = createSearxngSearchTool({ baseUrl: useRuntimeConfig().searxngBaseUrl })
            searchResultText = await searxngSearchTool.invoke({ query: cleanedText })
          } catch (err) {
            searchResultText = `Error: ${(err as Error).message}`
          }

          const invocation = parts.find(p => p.type === 'dynamic-tool' && p.toolCallId === toolCallId)
          if (invocation) {
            invocation.state = 'output-available'
            invocation.output = searchResultText
          }
          writer.write({
            type: 'tool-output-available',
            toolCallId,
            output: searchResultText,
            dynamic: true
          })

          const inputMessages = convertToLangchainMessages(
            uiMessages,
            `${cleanedText}\n\n[Search results for "${cleanedText}"]:\n${searchResultText}`
          )

          textIndex++
          writer.write({ type: 'text-start', id: `text-${textIndex}` })
          const stream = await baseModel.stream(inputMessages)
          for await (const chunk of stream) {
            const delta = typeof chunk.content === 'string' ? chunk.content : ''
            if (delta) {
              writer.write({ type: 'text-delta', id: `text-${textIndex}`, delta })
              currentText += delta
            }
          }
          writer.write({ type: 'text-end', id: `text-${textIndex}` })
          if (currentText) parts.push({ type: 'text', text: currentText })

          try {
            await onEnd(parts)
          } catch (err) {
            console.error('[langgraph onEnd] failed', err)
          }
          return
        }

        const langgraphTools = buildLanggraphTools({ workspacePath })
        const agent = createAgent({ model: baseModel, tools: langgraphTools })

        const inputMessages = convertToLangchainMessages(uiMessages, cleanedText)
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
