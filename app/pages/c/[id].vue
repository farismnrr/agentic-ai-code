<script setup lang="ts">
import { getTextFromMessage } from '@nuxt/ui/utils/ai'
import { models } from '~/utils/fixtures/models'

const route = useRoute()
const toast = useToast()

const { get, update, titleFrom } = useConversations()
const { take: takePendingPrompt } = usePendingPrompt()

const conversationId = computed(() => String(route.params.id))
const conversation = computed(() => get(conversationId.value))

useSeoMeta({ title: () => conversation.value?.title ?? 'Chat' })

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
      <UDashboardNavbar :title="conversation?.title ?? 'Chat'">
        <template #leading>
          <UDashboardSidebarToggle />
          <UDashboardSidebarCollapse />
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
          :assistant="{ actions: [] }"
        >
          <template #content="{ message }">
            <ChatMessageParts :message="message" />
          </template>

          <template #indicator>
            <UChatShimmer text="Thinking…" />
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
              v-if="message.role === 'assistant'"
              icon="i-lucide-refresh-cw"
              color="neutral"
              variant="ghost"
              size="xs"
              aria-label="Regenerate"
              @click="regenerate()"
            />
          </template>
        </UChatMessages>

        <!-- Lives inside a named slot deliberately: a bare child alongside
             #header/#body/#footer is treated as default-slot content and
             makes Vue drop the named slots. It's a teleported modal, so its
             position in the tree has no visual effect. -->
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
          :error="error"
          autofocus
          placeholder="Message AI Code…"
          @submit="submit"
        >
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
            <ChatToolPicker v-model="enabledToolIds" />
          </template>
        </UChatPrompt>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
