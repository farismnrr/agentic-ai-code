<script setup lang="ts">
import { chatModeItems, modelSupportsReasoning, reasoningEffortItems } from '../../utils/chat-options'

const route = useRoute()
const toast = useToast()

const { get, loadOne, update, titleFrom } = useConversations()
const { take: takePendingPrompt } = usePendingPrompt()

const conversationId = computed(() => String(route.params.id))
const conversation = computed(() => get(conversationId.value))

const { models, load: loadModels } = useModels()

const loadError = ref<Error | null>(null)

async function fetchInitialData() {
  loadError.value = null
  try {
    if (models.value.length === 0) {
      await loadModels()
    }
    // The conversations list only carries metadata, not messages (see
    // server/api/conversations/index.get.ts) — this page needs the full
    // conversation, and it must resolve before useConversationChat() reads
    // conversation.value.messages as the chat's initial seed. useChat() re-seeds
    // and rebuilds its chat instance any time that reactive value changes, so
    // loading it after mount would reset state instead of restoring it.
    //
    // The layout (app/layouts/default.vue) already resolves this conversation
    // and syncs the active workspace before its own sidebar renders — this call
    // is idempotent with that one, kept here because this page can't assume the
    // layout always ran it (and needs the awaited result regardless).
    await loadOne(conversationId.value)
  } catch (err) {
    loadError.value = err as Error
  }
}

await fetchInitialData()

const { get: getWorkspace } = useWorkspaces()
const workspaceName = computed(() => {
  if (!conversation.value?.workspaceId) return null
  return getWorkspace(conversation.value.workspaceId)?.name
})

useSeoMeta({ title: () => conversation.value?.title ?? 'Chat' })

const settings = useSettings()

const {
  messages,
  status,
  error,
  sendMessage,
  stop,
  regenerate,
  addToolApprovalResponse
} = useConversationChat(conversation)

// UChatMessages' own #indicator only shows while status === 'submitted', or
// while streaming with zero parts on the last message — the instant the
// first tool call starts (e.g. terminal exploring a workspace), that parts
// array is non-empty and the indicator disappears even though several more
// tool calls and the final answer are usually still to come. Each tool
// card does shimmer individually, but across a longer multi-step chain that
// reads as "stuck" rather than "still working" at a glance.
const isWorkingWithoutAnswerYet = computed(() => {
  if (status.value !== 'streaming') return false
  const last = messages.value[messages.value.length - 1]
  if (!last || last.role !== 'assistant') return false
  return !last.parts?.some(part => part.type === 'text' && part.text)
})

const input = ref('')

const { editorRef, syncText, clearEditor, handleKeydown, mentionItems } = useChatEditor(input, computed(() => settings.value.sendOnEnter))

const { modelId, mode, reasoningEffort, enabledToolIds } = useConversationConfiguration(conversation, models)

async function handleApprovalAnswer({ id, approved, toolId, remember }: { id: string, approved: boolean, toolId?: string, remember?: 'always' | 'never' }) {
  if (remember && toolId && conversation.value) {
    await update(conversation.value.id, {
      approvals: { ...conversation.value.approvals, [toolId]: remember }
    })
  }
  addToolApprovalResponse({ id, approved })
}

function updateApprovals(approvals: Record<string, 'always' | 'never'>) {
  if (conversation.value) {
    update(conversation.value.id, { approvals })
  }
}

const modelItems = computed(() =>
  models.value.map(model => ({ label: model.label, value: model.id, icon: 'i-lucide-box' }))
)

const modeItems = chatModeItems

const effortItems = reasoningEffortItems

const supportsReasoning = computed(() => {
  return modelSupportsReasoning(models.value.find(m => m.id === modelId.value))
})

function submit() {
  const text = input.value.trim()
  if (!text) return
  clearEditor()
  send(text)
}

function send(text: string) {
  void sendMessage({ text })
  // A conversation created from the empty state carries a placeholder title
  // until its first message names it.
  if (conversation.value && conversation.value.title === 'New chat') {
    update(conversation.value.id, { title: titleFrom(text) })
  }
}

/**
 * Edit a prompt and send it again. Truncating at the edited message rather
 * than appending is what makes it an edit — everything after it was an answer
 * to the old wording, so keeping it would leave the thread self-contradictory.
 */
function editAndResend(messageId: string, text: string) {
  const index = messages.value.findIndex(m => m.id === messageId)
  if (index === -1) return
  messages.value = messages.value.slice(0, index)
  send(text)
}

const editing = ref<{ id: string, text: string } | null>(null)

/** Feedback has nowhere to go without a backend, so it only acknowledges. */
const feedback = ref<Record<string, 'up' | 'down'>>({})

function rate(messageId: string, value: 'up' | 'down') {
  feedback.value = { ...feedback.value, [messageId]: value }
  toast.add({ title: 'Thanks for the feedback', icon: 'i-lucide-check', color: 'neutral' })
}

// A conversation opened straight from the empty state has its first prompt
// waiting; send it once, on mount, so a refresh doesn't replay it.
onMounted(() => {
  const pending = takePendingPrompt(conversationId.value)
  if (pending) send(pending)
})

// Esc halts a streaming reply, matching the stop button.
defineShortcuts({
  escape: {
    usingInput: true,
    handler: () => {
      if (status.value === 'streaming' || status.value === 'submitted') void stop()
    }
  }
})
</script>

<template>
  <UDashboardPanel :id="`chat-${conversationId}`">
    <template #header>
      <UDashboardNavbar>
        <template #left>
          <UDashboardSidebarCollapse />
          <div class="flex items-center gap-2">
            <h1 class="font-semibold text-default truncate">
              {{ conversation?.title ?? 'Chat' }}
            </h1>
            <UBadge
              v-if="workspaceName"
              variant="subtle"
              size="xs"
              color="neutral"
              class="hidden sm:inline-flex rounded-full"
            >
              {{ workspaceName }}
            </UBadge>
          </div>
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <div
        v-if="loadError"
        class="flex flex-1 items-center justify-center p-6"
      >
        <DataLoadError
          title="Couldn't load conversation"
          description="Failed to load conversation details or models."
          @retry="fetchInitialData()"
        />
      </div>

      <div
        v-else-if="!conversation"
        class="flex flex-1 items-center justify-center"
      >
        <UAlert
          icon="i-lucide-message-square-off"
          title="Conversation not found"
          description="It may have been deleted, or the link is stale."
          color="neutral"
          variant="subtle"
          class="max-w-md"
        />
      </div>

      <UContainer v-else>
        <div
          v-if="!messages.length && status === 'ready'"
          class="flex flex-1 items-center justify-center py-16"
        >
          <p class="text-muted">
            Send a message to get started.
          </p>
        </div>

        <UChatMessages
          v-else
          :messages="messages"
          :status="status"
          :assistant="{ actions: [], ui: { root: 'animate-message-in' } }"
          :user="{ ui: { root: 'animate-message-in' } }"
          class="max-w-3xl mx-auto w-full"
        >
          <template #content="{ message }">
            <ChatMessageParts :message="message" />
          </template>

          <template #indicator>
            <UChatShimmer
              text="Thinking…"
              class="animate-pulse"
            />
          </template>

          <template #actions="{ message }">
            <ChatMessageActions
              :message="message"
              :feedback="feedback[message.id]"
              @edit="editing = $event"
              @regenerate="regenerate()"
              @rate="rate(message.id, $event)"
            />
          </template>
        </UChatMessages>

        <div
          v-if="isWorkingWithoutAnswerYet"
          class="max-w-3xl mx-auto w-full px-4 pb-2"
        >
          <UChatShimmer
            text="Still working…"
            class="animate-pulse"
          />
        </div>

        <!-- Lives inside a named slot deliberately: a bare child alongside
             #header/#body/#footer is treated as default-slot content and
             makes Vue drop the named slots. It's a teleported modal, so its
             position in the tree has no visual effect. -->
        <ChatEditMessageModal
          v-model="editing"
          @send="editAndResend($event.id, $event.text)"
        />

        <ChatToolApproval
          :messages="messages"
          :conversation="conversation"
          @answer="handleApprovalAnswer"
        />
      </UContainer>
    </template>

    <template
      v-if="conversation"
      #footer
    >
      <UContainer class="pb-4 sm:pb-6">
        <UChatPrompt
          v-model="input"
          :error="error"
          :ui="{ footer: 'flex-wrap sm:flex-nowrap justify-start' }"
          @submit="submit"
        >
          <template #body="{ submit: promptSubmit, disabled }">
            <UEditor
              ref="editorRef"
              v-slot="{ editor }"
              autofocus
              placeholder="Message AI Code…"
              :editable="!disabled"
              :mention="mode === 'chat'"
              class="w-full bg-transparent min-h-[44px]"
              :editor-props="{ handleKeyDown: (_view, event) => handleKeydown(event, promptSubmit) }"
              @update:model-value="syncText()"
            >
              <UEditorMentionMenu
                v-if="mode === 'chat'"
                :editor="editor"
                :items="mentionItems"
              />
            </UEditor>
          </template>

          <UChatPromptSubmit
            :status="status"
            @stop="stop()"
            @reload="regenerate()"
          />

          <template #footer>
            <ChatConfigControls
              v-model:model-id="modelId"
              v-model:mode="mode"
              v-model:reasoning-effort="reasoningEffort"
              v-model:enabled-tool-ids="enabledToolIds"
              :model-items="modelItems"
              :mode-items="modeItems"
              :effort-items="effortItems"
              :supports-reasoning="supportsReasoning"
              :show-tools="mode === 'agent'"
              :approvals="conversation?.approvals"
              @update-approvals="updateApprovals"
            />
            <ChatContextUsage
              :conversation="conversation"
              :model-id="modelId"
            />
          </template>
        </UChatPrompt>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
