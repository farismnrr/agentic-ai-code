import { generateText, convertToModelMessages, type LanguageModel } from 'ai'
import type { UIMessage } from '#shared/types/chat'
import { conversations as conversationsTable } from '../database/schema'
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
    summaryMessage = {
      id: 'summary-' + conv.id,
      role: 'system',
      createdAt: new Date(),
      parts: [{ type: 'text', text: `Conversation summary so far: ${conv.contextSummary}` }]
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
    const modelMessages = await convertToModelMessages(messagesToSummarize)
    const { text: newSummary } = await generateText({
      model: summarizerModel,
      system: systemPrompt,
      messages: modelMessages
    })

    if (newSummary) {
      const db = useDb()
      // Persist new summary and cutoff
      await db.update(conversationsTable)
        .set({
          contextSummary: newSummary,
          contextSummaryUpToMessageId: newCutoffMessage.id,
          updatedAt: new Date()
        })
        .where(eq(conversationsTable.id, conv.id))

      const newSummaryMessage: UIMessage = {
        id: 'summary-' + conv.id + '-' + Date.now(),
        role: 'system',
        createdAt: new Date(),
        parts: [{ type: 'text', text: `Conversation summary so far: ${newSummary}` }]
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
