import type { Conversation } from '#shared/types/chat'
import { friendlyRequestErrorMessage } from '../utils/chat-errors'

export function useNewChatController(input: Ref<string>, workspaceId: Ref<string | undefined>, modelId: Ref<string | undefined>, mode: Ref<Conversation['mode']>, reasoningEffort: Ref<NonNullable<Conversation['reasoningEffort']>>, enabledToolIds: Ref<string[]>, permissionMode: Ref<Conversation['permissionMode']>) {
  const { create, update, titleFrom } = useConversations()
  const { activeWorkspaceId, setActive } = useWorkspaces()
  const { set: setPendingPrompt } = usePendingPrompt()
  const router = useRouter()
  const toast = useToast()

  async function start(text: string) {
    const trimmed = text.trim()
    if (!trimmed) return
    try {
      const conversation = await create({ title: titleFrom(trimmed), modelId: modelId.value, mode: mode.value, reasoningEffort: reasoningEffort.value, permissionMode: permissionMode.value, workspaceId: workspaceId.value })
      if (enabledToolIds.value.length > 0) await update(conversation.id, { enabledToolIds: enabledToolIds.value })
      input.value = ''
      if (workspaceId.value && workspaceId.value !== activeWorkspaceId.value) setActive(workspaceId.value)
      setPendingPrompt(conversation.id, trimmed)
      void router.push(`/chat/${conversation.id}`)
    } catch (err) {
      toast.add({ title: 'Failed to start conversation', description: friendlyRequestErrorMessage(err), color: 'error' })
    }
  }

  return { start }
}
