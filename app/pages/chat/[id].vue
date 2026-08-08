<script setup lang="ts">
import { getTextFromMessage } from '@nuxt/ui/utils/ai'
import { models } from '#shared/utils/models'

const route = useRoute()
const toast = useToast()

const { get, loadOne, update, titleFrom } = useConversations()
const { take: takePendingPrompt } = usePendingPrompt()

const conversationId = computed(() => String(route.params.id))
const conversation = computed(() => get(conversationId.value))

const { get: getWorkspace } = useWorkspaces()
const workspaceName = computed(() => {
  if (!conversation.value?.workspaceId) return null
  return getWorkspace(conversation.value.workspaceId)?.name
})

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

const input = ref('')

const forceCloseMention = ref(false)
watch(input, () => {
  forceCloseMention.value = false
})
const mentionMatch = computed(() => forceCloseMention.value ? null : input.value.match(/(?:^|\\s)@(\\w*)$/))
const mentionOpen = computed(() => mentionMatch.value !== null)
const mentionFilter = computed(() => mentionMatch.value ? mentionMatch.value[1]! : '')

function onMentionSelect(trigger: string) {
  if (!mentionMatch.value) return
  const matchStr = mentionMatch.value[0]
  const replaceStr = matchStr.replace(/@\\w*$/, `@${trigger} `)
  input.value = input.value.substring(0, input.value.length - matchStr.length) + replaceStr
  forceCloseMention.value = true
  setTimeout(() => {
    const textarea = document.querySelector('textarea')
    if (textarea) textarea.focus()
  }, 0)
}
const enabledToolIds = computed({
  get: () => conversation.value?.enabledToolIds ?? [],
  set: (value: string[]) => {
    if (conversation.value) update(conversation.value.id, { enabledToolIds: value })
  }
})

function rememberApproval({ toolId, decision }: { toolId: string, decision: 'always' | 'never' }) {
  if (!conversation.value) return
  update(conversation.value.id, {
    approvals: { ...conversation.value.approvals, [toolId]: decision }
  })
}

const modelItems = computed(() =>
  models.map(model => ({ label: model.label, value: model.id, icon: model.icon }))
)

const modelId = computed({
  get: () => conversation.value?.modelId ?? models[0]!.id,
  set: (value: string) => {
    if (conversation.value) update(conversation.value.id, { modelId: value })
  }
})

const mode = computed({
  get: () => conversation.value?.mode ?? 'chat',
  set: (value: 'chat' | 'agent') => {
    if (conversation.value) update(conversation.value.id, { mode: value })
  }
})

const reasoningEffort = computed({
  get: () => conversation.value?.reasoningEffort ?? 'medium',
  set: (value: 'low' | 'medium' | 'high' | 'max') => {
    if (conversation.value) update(conversation.value.id, { reasoningEffort: value })
  }
})

const effortItems = [
  { label: 'Low Effort', value: 'low' },
  { label: 'Medium Effort', value: 'medium' },
  { label: 'High Effort', value: 'high' },
  { label: 'Max Effort', value: 'max' }
]

const supportsReasoning = computed(() => {
  return models.find(m => m.id === modelId.value)?.supportsReasoning ?? false
})

function submit() {
  const text = input.value.trim()
  if (!text) return
  input.value = ''
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

async function copy(text: string) {
  await navigator.clipboard.writeText(text)
  toast.add({ title: 'Copied', icon: 'i-lucide-check', color: 'success' })
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

function confirmEdit() {
  const pending = editing.value
  if (!pending?.text.trim()) return
  editing.value = null
  editAndResend(pending.id, pending.text.trim())
}

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
        v-if="!conversation"
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
            <UButton
              icon="i-lucide-copy"
              color="neutral"
              variant="ghost"
              size="xs"
              aria-label="Copy message"
              @click="copy(getTextFromMessage(message))"
            />

            <UButton
              v-if="message.role === 'user'"
              icon="i-lucide-pencil"
              color="neutral"
              variant="ghost"
              size="xs"
              aria-label="Edit and resend"
              @click="editing = { id: message.id, text: getTextFromMessage(message) }"
            />

            <template v-if="message.role === 'assistant'">
              <UButton
                icon="i-lucide-refresh-cw"
                color="neutral"
                variant="ghost"
                size="xs"
                aria-label="Regenerate"
                @click="regenerate()"
              />
              <UButton
                icon="i-lucide-thumbs-up"
                :color="feedback[message.id] === 'up' ? 'primary' : 'neutral'"
                variant="ghost"
                size="xs"
                aria-label="Good response"
                @click="rate(message.id, 'up')"
              />
              <UButton
                icon="i-lucide-thumbs-down"
                :color="feedback[message.id] === 'down' ? 'error' : 'neutral'"
                variant="ghost"
                size="xs"
                aria-label="Bad response"
                @click="rate(message.id, 'down')"
              />
            </template>
          </template>
        </UChatMessages>

        <!-- Lives inside a named slot deliberately: a bare child alongside
             #header/#body/#footer is treated as default-slot content and
             makes Vue drop the named slots. It's a teleported modal, so its
             position in the tree has no visual effect. -->
        <UModal
          :open="editing !== null"
          title="Edit message"
          description="Everything after this message will be replaced."
          @update:open="editing = null"
        >
          <template #body>
            <UTextarea
              v-if="editing"
              v-model="editing.text"
              :rows="4"
              autoresize
              autofocus
              class="w-full"
            />
          </template>

          <template #footer>
            <div class="flex w-full justify-end gap-2">
              <UButton
                label="Cancel"
                color="neutral"
                variant="ghost"
                @click="editing = null"
              />
              <UButton
                label="Send"
                @click="confirmEdit"
              />
            </div>
          </template>
        </UModal>

        <ChatToolApproval
          :messages="messages"
          :conversation="conversation"
          @respond="addToolApprovalResponse"
          @remember="rememberApproval"
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
          :submit-on-enter="settings.sendOnEnter"
          :error="error"
          autofocus
          placeholder="Message AI Code…"
          :ui="{ footer: 'flex-wrap sm:flex-nowrap justify-start' }"
          @submit="submit"
        >
          <template #header>
            <ChatMentionMenu
              v-if="mode === 'chat'"
              :open="mentionOpen"
              :filter="mentionFilter"
              @select="onMentionSelect"
              @close="forceCloseMention = true"
            />
          </template>

          <UChatPromptSubmit
            :status="status"
            @stop="stop()"
            @reload="regenerate()"
          />

          <template #footer>
            <USelect
              v-model="modelId"
              :items="modelItems"
              :icon="models.find(m => m.id === modelId)?.icon"
              variant="ghost"
              size="sm"
            />
            <USelect
              v-if="supportsReasoning"
              v-model="reasoningEffort"
              :items="effortItems"
              variant="ghost"
              size="sm"
            />
            <ChatToolPicker
              v-if="mode === 'agent'"
              v-model="enabledToolIds"
            />
          </template>
        </UChatPrompt>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
