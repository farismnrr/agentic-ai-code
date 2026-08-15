import type { Conversation } from '#shared/types/chat'

export interface ConversationGroup {
  label: string
  conversations: Conversation[]
}

const DAY = 24 * 60 * 60 * 1000

/**
 * Bucket conversations the way chat apps do — by recency, relative to now
 * rather than to calendar boundaries, so the labels stay honest at 2am.
 */
export function groupConversations(
  conversations: Conversation[],
  now = Date.now()
): ConversationGroup[] {
  const buckets: ConversationGroup[] = [
    { label: 'Today', conversations: [] },
    { label: 'Yesterday', conversations: [] },
    { label: 'Previous 7 days', conversations: [] },
    { label: 'Previous 30 days', conversations: [] },
    { label: 'Older', conversations: [] }
  ]

  for (const conversation of conversations) {
    const age = now - conversation.updatedAt
    const bucket
      = age < DAY
        ? buckets[0]
        : age < 2 * DAY
          ? buckets[1]
          : age < 7 * DAY
            ? buckets[2]
            : age < 30 * DAY
              ? buckets[3]
              : buckets[4]
    bucket!.conversations.push(conversation)
  }

  return buckets.filter(group => group.conversations.length > 0)
}
