import { logger } from '../../observability/logger'
import { createAgent } from 'langchain'

import { buildLanggraphTools } from './langgraph-tools'
import type { UIMessage } from '#shared/types/chat'
import { createUIMessageStream, getToolName } from 'ai'
import { HumanMessage, AIMessage, SystemMessage, ToolMessage } from '@langchain/core/messages'
import type { getLanggraphModel } from '../providers/langgraph-model'
import type { RequestTelemetryContext } from '../../../application/observability/contracts'

type LanggraphModel = ReturnType<typeof getLanggraphModel>

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
  const resultMessages: (HumanMessage | AIMessage | SystemMessage | ToolMessage)[] = []

  uiMessages.forEach((m, index) => {
    let textContent = ''
    const toolCalls: { id: string, name: string, args: Record<string, unknown> }[] = []
    const toolResults: { toolCallId: string, output: string }[] = []

    if (index === uiMessages.length - 1 && cleanedLastText !== undefined) {
      textContent = cleanedLastText
    } else if (m.parts) {
      for (const part of m.parts) {
        if (part.type === 'text') {
          textContent += part.text
        } else if (part.type === 'dynamic-tool' || part.type === 'tool-terminal') {
          // Both branches share `toolCallId`/`input` in ai@7's real
          // ToolUIPart/DynamicToolUIPart shape — `.args`/`.id`/`.command`
          // never existed on either (a leftover from an older SDK shape);
          // `getToolName` is the SDK's own helper for the name, since the
          // static branch (`tool-terminal`) carries it in `part.type`
          // rather than a separate field.
          const rawInput = part.input
          const toolCallId = part.toolCallId || `call_${index}`
          const toolName = getToolName(part)
          if (rawInput) {
            toolCalls.push({
              id: toolCallId,
              name: toolName,
              args: (typeof rawInput === 'object' && rawInput !== null) ? rawInput as Record<string, unknown> : { input: rawInput }
            })
          }
          if (part.output !== undefined) {
            toolResults.push({
              toolCallId: toolCallId,
              output: typeof part.output === 'string' ? part.output : JSON.stringify(part.output)
            })
          }
        }
      }
    }

    if (m.role === 'user') {
      resultMessages.push(new HumanMessage(textContent))
    } else if (m.role === 'system') {
      resultMessages.push(new SystemMessage(textContent))
    } else {
      // assistant
      if (toolCalls.length > 0) {
        resultMessages.push(new AIMessage({
          content: textContent,
          tool_calls: toolCalls
        }))
        for (const tr of toolResults) {
          resultMessages.push(new ToolMessage({
            content: tr.output,
            tool_call_id: tr.toolCallId
          }))
        }
      } else {
        resultMessages.push(new AIMessage(textContent))
      }
    }
  })

  return resultMessages
}

export function runLanggraphChat({
  uiMessages,
  baseModel,
  systemPrompt,
  abortSignal,
  cleanup,
  onEnd,
  telemetry
}: {
  uiMessages: UIMessage[]
  baseModel: LanggraphModel
  systemPrompt: string | undefined
  abortSignal?: AbortSignal
  cleanup: () => Promise<void>
  onEnd: (parts: UIMessage['parts'], totalTokens?: number) => Promise<void>
  telemetry?: RequestTelemetryContext
}) {
  const { forced, cleanedText } = extractForcedSearch(uiMessages)

  return createUIMessageStream({
    async execute({ writer }) {
      const parts: UIMessage['parts'] = []
      let currentText = ''
      let textIndex = 0
      // Declared outside the try so the catch block below can clear it too —
      // a `const` inside `try {}` isn't visible in the paired `catch {}`.
      let timeoutId: ReturnType<typeof setTimeout> | undefined
      let totalTokens: number | undefined
      let langgraphTools: Awaited<ReturnType<typeof buildLanggraphTools>> | undefined

      try {
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
            langgraphTools = await buildLanggraphTools()
            searchResultText = await langgraphTools.search(cleanedText)
          } catch (err) {
            // Plan 035 remediation round 2 — same class of leak as the MCP
            // tool-result fix: this text becomes user-visible chat content
            // (not an HTTP error path), so raw err.message (which can carry
            // internal hostnames/ports/provider detail) must never surface
            // here. Generic message to the client; raw cause to private
            // observability only.
            telemetry?.error('chat.tool.search.dispatch', 'searxng_search_failed', err)
            searchResultText = 'Search failed'
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
            if (chunk.usage_metadata?.total_tokens) {
              totalTokens = chunk.usage_metadata.total_tokens
            }
          }
          writer.write({ type: 'text-end', id: `text-${textIndex}` })
          if (currentText) parts.push({ type: 'text', text: currentText })

          try {
            await onEnd(parts, totalTokens)
          } catch (err) {
            logger.error('[langgraph onEnd] failed', err)
          }
          return
        }

        langgraphTools = await buildLanggraphTools()
        const agent = createAgent({ model: baseModel, tools: langgraphTools.tools })

        const inputMessages = convertToLangchainMessages(uiMessages, cleanedText)
        if (systemPrompt) inputMessages.unshift(new SystemMessage(systemPrompt))
        // Without a cap, a model that keeps mis-forming tool calls can loop
        // indefinitely instead of ever producing a final answer — agent mode
        // already caps this via streamText's `stopWhen: stepCountIs(5)`.
        // LangGraph counts each model call AND each tool call as a step, so
        // this needs enough headroom for genuine multi-step exploration
        // (e.g. "explain this project" reasonably takes 4-6 tool calls)
        // plus the final synthesis call — 10 cuts that off mid-exploration.
        // LangChain has no built-in overall timeout (unlike streamText's
        // native `timeout` option used in agent mode) — without this, a
        // hung model API call had nothing to time it out, and the request
        // would wait indefinitely. A tool call itself is already capped by
        // execa's own 30s timeout; this bounds the model-call side.
        const timeoutController = new AbortController()
        timeoutId = setTimeout(() => timeoutController.abort(new Error('Model call timed out after 180s')), 180_000)
        const abortParent = () => timeoutController.abort()
        abortSignal?.addEventListener('abort', abortParent, { once: true })
        const stream = agent.streamEvents(
          { messages: inputMessages },
          { version: 'v2', recursionLimit: 25, signal: timeoutController.signal }
        )

        for await (const event of stream) {
          if (event.event === 'on_chat_model_stream') {
            const chunk = event.data?.chunk?.content
            const chunkUsage = event.data?.chunk?.usage_metadata
            if (chunkUsage?.total_tokens) {
              totalTokens = chunkUsage.total_tokens
            }
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

        clearTimeout(timeoutId)
        abortSignal?.removeEventListener('abort', abortParent)

        if (currentText) {
          writer.write({ type: 'text-end', id: `text-${textIndex}` })
          parts.push({ type: 'text', text: currentText })
        }

        try {
          await onEnd(parts, totalTokens)
        } catch (err) {
          logger.error('[langgraph onEnd] failed', err)
        }
      } catch (e: unknown) {
        clearTimeout(timeoutId)
        if (currentText) {
          writer.write({ type: 'text-end', id: `text-${textIndex}` })
          parts.push({ type: 'text', text: currentText })
        }
        const cancelled = abortSignal?.aborted === true
        const errorText = cancelled ? 'Request cancelled' : 'Tool execution failed'
        if (cancelled) telemetry?.event('chat.stream.chunk_error', 'cancelled')
        else telemetry?.error('chat.stream.chunk_error', 'chat_stream_error', e)
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
            await onEnd(parts, totalTokens)
          } catch (err) {
            logger.error('[langgraph onEnd] failed', err)
          }
        }
      } finally {
        await langgraphTools?.close()
        await cleanup()
      }
    }
  })
}
