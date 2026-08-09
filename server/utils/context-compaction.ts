import { generateText, convertToModelMessages, type LanguageModel } from 'ai'
import type { UIMessage } from '#shared/types/chat'
import { conversations as conversationsTable, messages as messagesTable } from '../database/schema'
import { eq, inArray } from 'drizzle-orm'
import { logger } from './logger'

// Token estimation heuristic: approximate tokens based on stringified length
const estimateTokens = (obj: unknown): number => {
  return Math.ceil(JSON.stringify(obj).length / 4)
}

interface ResolveMessagesForModelParams {
  messages: UIMessage[]
  conv: { id: string, contextSummary: string | null, contextSummaryUpToMessageId: string | null }
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

  const tokenMap: Record<string, number> = {}
  try {
    const db = useDb()
    const msgIds = candidate.map(m => m.id).filter(id => id && !id.startsWith('temp-') && !id.startsWith('summary-'))
    if (msgIds.length > 0) {
      const usages = await db.select({ id: messagesTable.id, totalTokens: messagesTable.totalTokens })
        .from(messagesTable)
        .where(inArray(messagesTable.id, msgIds))

      for (const u of usages) {
        if (u.totalTokens != null) tokenMap[u.id] = u.totalTokens
      }
    }
  } catch (err) {
    logger.warn('[compaction] Failed to fetch totalTokens', err)
  }

  let measuredBaseline = 0
  let measuredIdx = -1

  for (let i = candidate.length - 1; i >= 0; i--) {
    const m = candidate[i]
    if (m.role === 'assistant' && m.id && tokenMap[m.id] != null) {
      measuredBaseline = tokenMap[m.id]
      measuredIdx = i
      break
    }
  }

  let currentTokens: number
  if (measuredIdx >= 0) {
    const unmeasured = candidate.slice(measuredIdx + 1)
    currentTokens = measuredBaseline + estimateTokens(unmeasured)
  } else {
    currentTokens = estimateTokens(candidate)
  }

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

  const systemPrompt = 'Summarize the conversation so far, preserving key facts, decisions, file/code references, and open tasks. Be concise.'

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

      return [newSummaryMessage, ...tail]
    }
  } catch (err) {
    logger.error('[compaction] Failed to generate summary', err)
  }

  return candidate
}
