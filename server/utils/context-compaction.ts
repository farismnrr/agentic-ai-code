import { streamText, convertToModelMessages, type LanguageModel } from 'ai'
import type { UIMessage } from '#shared/types/chat'
import { conversations as conversationsTable, messages as messagesTable } from '../database/schema'
import { eq } from 'drizzle-orm'
import { logger } from './logger'

// Token estimation heuristic: approximate tokens based on stringified length
const estimateTokens = (obj: unknown): number => {
  return Math.ceil(JSON.stringify(obj).length / 4)
}

// Last-resort safety net for when the tail alone (the messages a
// compaction pass deliberately leaves untouched) is still over budget —
// e.g. one huge tool output/file read sitting in the kept tail. Clips
// oversized text/tool-output content within each part, oldest tail
// message first, leaving the single most recent message untouched since
// it's the one most likely to be referenced immediately. Never mutates
// the input messages/parts.
const MAX_PART_CHARS = 4000

function truncatePartsIfNeeded(tail: UIMessage[], summaryMessage: UIMessage, budget: number): UIMessage[] {
  if (estimateTokens([summaryMessage, ...tail]) <= budget) return tail

  const clipped = tail.map(m => ({
    ...m,
    parts: m.parts.map(p => ({ ...p }))
  }))

  for (let i = 0; i < clipped.length - 1; i++) {
    const message = clipped[i]
    if (!message) continue
    for (const part of message.parts) {
      for (const key of ['text', 'input', 'output'] as const) {
        const value = (part as Record<string, unknown>)[key]
        if (typeof value === 'string' && value.length > MAX_PART_CHARS) {
          (part as Record<string, unknown>)[key] = `${value.slice(0, MAX_PART_CHARS)}\n…[truncated ${value.length - MAX_PART_CHARS} chars — context budget safety net]`
        }
      }
    }
    if (estimateTokens([summaryMessage, ...clipped]) <= budget) break
  }

  return clipped
}

// The summarizer call carries no `tools`, so raw tool-call/tool-result
// parts passed straight through `convertToModelMessages` land in its
// prompt as the provider's native tool-call wire format with nothing
// registered to handle it. Confirmed against production: a weaker model
// (the "Free Models" provider) pattern-matched that shape in its own
// input and echoed pseudo tool-call syntax back as its entire "summary"
// instead of prose. Flatten tool parts to plain descriptive text first so
// the summarizer only ever sees natural language.
function flattenToolPartsForSummary(msgs: UIMessage[]): UIMessage[] {
  return msgs.map((m) => {
    const hasToolPart = m.parts.some(p => String(p.type).startsWith('tool-') || p.type === 'dynamic-tool')
    if (!hasToolPart) return m
    return {
      ...m,
      parts: m.parts.map((p) => {
        if (!(String(p.type).startsWith('tool-') || p.type === 'dynamic-tool')) return p
        const part = p as Record<string, unknown>
        const name = 'toolName' in part ? part.toolName : String(p.type).replace(/^tool-/, '')
        const input = 'input' in part ? JSON.stringify(part.input) : undefined
        const output = 'output' in part ? JSON.stringify(part.output) : undefined
        return {
          type: 'text',
          text: `[Used tool "${name}"${input ? ` with input ${input}` : ''}${output ? ` — result: ${output}` : ''}]`
        }
      })
    }
  })
}

interface ResolveMessagesForModelParams {
  messages: UIMessage[]
  conv: {
    id: string
    contextSummary: string | null
    contextSummaryUpToMessageId: string | null
    lastMeasuredTokens: number | null
    lastMeasuredMessageId: string | null
  }
  contextWindow: number | null | undefined
  maxOutputTokens: number | null | undefined
  getSummarizerModel: () => LanguageModel
}

export async function resolveMessagesForModel({
  messages,
  conv,
  contextWindow,
  maxOutputTokens,
  getSummarizerModel
}: ResolveMessagesForModelParams): Promise<UIMessage[]> {
  if (!contextWindow) {
    return messages
  }

  const outMax = maxOutputTokens ?? 4096
  const margin = Math.floor(contextWindow * 0.1) // 10% margin
  const budget = contextWindow - outMax - margin

  let candidate = messages
  let summaryMessage: UIMessage | null = null

  let cutoffIdx = -1
  if (conv.contextSummary) {
    // Not `role: 'system'` — confirmed against production (AI SDK v7):
    // `streamText()` flatly rejects any system-role entry inside
    // `messages`, throwing "System messages are not allowed in the prompt
    // or messages fields. Use the instructions option instead." The real
    // system prompt already goes through `system:`/`buildWorkspaceSystemPrompt`
    // separately in chat.post.ts; this is just conversation content, so
    // `role: 'user'` (valid in any position, for every provider) carries
    // it instead, framed clearly so the model doesn't mistake it for
    // something the human actually typed.
    summaryMessage = {
      id: 'summary-' + conv.id,
      role: 'user',
      createdAt: new Date(),
      parts: [{ type: 'text', text: `[Context note, not sent by the user — summary of the earlier conversation]: ${conv.contextSummary}` }]
    }

    if (conv.contextSummaryUpToMessageId) {
      cutoffIdx = messages.findIndex(m => m.id === conv.contextSummaryUpToMessageId)
    }

    if (cutoffIdx >= 0) {
      candidate = [summaryMessage, ...messages.slice(cutoffIdx + 1)]
    } else {
      candidate = [summaryMessage, ...messages]
    }
  }

  // Baseline comes straight off the already-loaded `conv` row (see
  // cacheLastMeasuredTokens in chat.post.ts) instead of a fresh `messages`
  // query — keeps the common "well under budget" case free of extra reads.
  // Only meaningful if that measured message is still inside `candidate`
  // (i.e. it wasn't itself folded into an even newer summary since).
  const measuredIdx = conv.lastMeasuredMessageId != null
    ? candidate.findIndex(m => m.id === conv.lastMeasuredMessageId)
    : -1

  const currentTokens = measuredIdx >= 0 && conv.lastMeasuredTokens != null
    ? conv.lastMeasuredTokens + estimateTokens(candidate.slice(measuredIdx + 1))
    : estimateTokens(candidate)

  if (currentTokens <= budget) {
    return candidate
  }

  // Need to compact
  // Keep the last ~6 messages untouched
  const keepTailCount = 6
  if (messages.length <= keepTailCount) {
    // Cannot compact further usefully
    return candidate
  }

  const newCutoffIdx = messages.length - keepTailCount - 1
  const newCutoffMessage = messages[newCutoffIdx]
  if (!newCutoffMessage) return candidate

  const messagesToSummarize = []
  if (summaryMessage) {
    messagesToSummarize.push(summaryMessage)
  }
  messagesToSummarize.push(...messages.slice(cutoffIdx + 1, newCutoffIdx + 1))

  const tail = messages.slice(newCutoffIdx + 1)

  logger.info('[compaction] Triggering conversation summary', { conversationId: conv.id, currentTokens, budget })

  // This prompt runs again on every subsequent compaction, each time fed
  // the previous summary as input — explicitly instructing it to carry
  // forward existing factual anchors (not just "be concise" generally)
  // keeps repeated rounds of re-summarization from gradually eroding
  // specifics that were already captured.
  const systemPrompt = 'Summarize the conversation so far, preserving key facts, decisions, file/code references, and open tasks. Be concise, but if a "Conversation summary so far" is included in the input, treat every concrete fact, decision, file path, identifier, and number already stated in it as required to carry forward verbatim into the new summary — do not drop or generalize existing details just to make room for new ones.'

  try {
    const summarizerModel = getSummarizerModel()
    const modelMessages = await convertToModelMessages(flattenToolPartsForSummary(messagesToSummarize))
    // Not `generateText()` — some OpenAI-compatible providers (confirmed
    // against the "Free Models" router in production: "Invalid JSON
    // response... Unexpected token 'd', \"data: {...\"") always respond
    // with an SSE stream regardless of the request's `stream` flag, which
    // breaks `generateText()`'s non-streaming JSON parse outright. The
    // main chat flow already talks to every configured provider
    // successfully because it always goes through `streamText()` — use
    // the same proven path here instead of a second, less-compatible one.
    const summaryResult = streamText({
      model: summarizerModel,
      system: systemPrompt,
      messages: modelMessages
    })
    const newSummary = await summaryResult.text

    if (newSummary) {
      const db = useDb()

      // Cache the cutoff message's createdAt alongside its id so
      // chat.post.ts can bound its per-turn history query
      // (`createdAt > this`) instead of fetching the whole conversation
      // every turn — this only runs on an actual compaction event (rare),
      // not the per-turn hot path.
      const [cutoffRow] = await db.select({ createdAt: messagesTable.createdAt })
        .from(messagesTable)
        .where(eq(messagesTable.id, newCutoffMessage.id))
        .limit(1)

      // Persist new summary and cutoff
      await db.update(conversationsTable)
        .set({
          contextSummary: newSummary,
          contextSummaryUpToMessageId: newCutoffMessage.id,
          contextSummaryUpToCreatedAt: cutoffRow?.createdAt,
          updatedAt: new Date()
        })
        .where(eq(conversationsTable.id, conv.id))

      const newSummaryMessage: UIMessage = {
        id: 'summary-' + conv.id + '-' + Date.now(),
        role: 'user',
        createdAt: new Date(),
        parts: [{ type: 'text', text: `[Context note, not sent by the user — summary of the earlier conversation]: ${newSummary}` }]
      }

      // Single compaction pass isn't guaranteed to land under budget on its
      // own — e.g. the kept tail alone can contain one outsized tool output.
      // Clip oversized part content within the tail (oldest tail message
      // first) as a last-resort safety net rather than silently shipping
      // an over-budget request.
      const safeTail = truncatePartsIfNeeded(tail, newSummaryMessage, budget)
      if (safeTail !== tail && estimateTokens([newSummaryMessage, ...safeTail]) > budget) {
        logger.warn('[compaction] Still over budget after summarize + truncate safety net', { conversationId: conv.id, budget })
      }

      return [newSummaryMessage, ...safeTail]
    }
  } catch (err) {
    logger.error('[compaction] Failed to generate summary', err)
  }

  return candidate
}
