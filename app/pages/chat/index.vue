<script setup lang="ts">
import { models } from '#shared/utils/models'

useSeoMeta({ title: 'New chat' })

const { create, titleFrom } = useConversations()
const settings = useSettings()
const { set: setPendingPrompt } = usePendingPrompt()
const router = useRouter()

const input = ref('')
// Seeded from the saved default so the settings page actually governs this.
const modelId = ref(settings.value.defaultModelId)

const suggestions = [
  { label: 'Search the repo for chat components', icon: 'i-lucide-folder-search' },
  { label: 'Why is there no tailwind.config.js?', icon: 'i-lucide-help-circle' },
  { label: 'Show me an example component', icon: 'i-lucide-code' },
  { label: 'List the open pull requests', icon: 'i-simple-icons-github' }
]

const modelItems = computed(() =>
  models.map(model => ({ label: model.label, value: model.id, icon: model.icon }))
)

async function start(text: string) {
  const trimmed = text.trim()
  if (!trimmed) return

  const conversation = await create({ title: titleFrom(trimmed), modelId: modelId.value })
  // The chat instance doesn't exist until the next page mounts, so hand the
  // prompt over rather than trying to send it here.
  setPendingPrompt(conversation.id, trimmed)
  void router.push(`/chat/${conversation.id}`)
}
</script>

<template>
  <UDashboardPanel id="home">
    <template #header>
      <UDashboardNavbar title="New chat">
        <template #left>
          <UDashboardSidebarCollapse />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <UContainer class="flex w-full flex-1 flex-col justify-center gap-8 py-10">
        <div class="text-center">
          <h1 class="text-2xl font-semibold text-highlighted sm:text-3xl">
            What are we building?
          </h1>
          <p class="mt-2 text-muted">
            Ask anything. Connected MCP tools are used when they help.
          </p>
        </div>

        <UChatPrompt
          v-model="input"
          :submit-on-enter="settings.sendOnEnter"
          autofocus
          placeholder="Message AI Code…"
          :ui="{ footer: 'flex-wrap sm:flex-nowrap' }"
          @submit="start(input)"
        >
          <UChatPromptSubmit />

          <template #footer>
            <USelect
              v-model="modelId"
              :items="modelItems"
              :icon="models.find(m => m.id === modelId)?.icon"
              variant="ghost"
              size="sm"
            />
          </template>
        </UChatPrompt>

        <div class="flex flex-wrap justify-center gap-2">
          <UButton
            v-for="suggestion in suggestions"
            :key="suggestion.label"
            :label="suggestion.label"
            :icon="suggestion.icon"
            color="neutral"
            variant="subtle"
            size="sm"
            @click="start(suggestion.label)"
          />
        </div>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
