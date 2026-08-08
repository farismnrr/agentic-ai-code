<script setup lang="ts">
import { models } from '#shared/utils/models'

useSeoMeta({ title: 'New chat' })

const { create, titleFrom } = useConversations()
const { loaded, activeWorkspaceId, workspaces, setActive } = useWorkspaces()
const settings = useSettings()
const { set: setPendingPrompt } = usePendingPrompt()
const router = useRouter()
const toast = useToast()

// Belt-and-suspenders alongside layouts/default.vue's own restore: pages
// and layouts each get their own Suspense boundary in Nuxt, so the
// layout finishing its async setup (and correctly restoring
// activeWorkspaceId there) doesn't guarantee this page's *own* render
// runs after it — this page has no async setup of its own to block on.
// Reactive and idempotent, so it's harmless if the layout already did it.
watchEffect(() => {
  if (loaded.value && !activeWorkspaceId.value && settings.value.lastActiveWorkspaceId) {
    const w = workspaces.value.find(w => w.id === settings.value.lastActiveWorkspaceId)
    if (w) setActive(w.id)
  }
})

const input = ref('')
// Seeded from the saved default so the settings page actually governs this.
const modelId = ref(settings.value.defaultModelId)
const reasoningEffort = ref<'low' | 'medium' | 'high' | 'max'>('medium')

const effortItems = [
  { label: 'Low Effort', value: 'low' },
  { label: 'Medium Effort', value: 'medium' },
  { label: 'High Effort', value: 'high' },
  { label: 'Max Effort', value: 'max' }
]

const supportsReasoning = computed(() => {
  return models.find(m => m.id === modelId.value)?.supportsReasoning ?? false
})

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

  try {
    const conversation = await create({ title: titleFrom(trimmed), modelId: modelId.value, reasoningEffort: reasoningEffort.value })
    // The chat instance doesn't exist until the next page mounts, so hand the
    // prompt over rather than trying to send it here.
    setPendingPrompt(conversation.id, trimmed)
    void router.push(`/chat/${conversation.id}`)
  } catch (err) {
    toast.add({
      title: 'Failed to start conversation',
      description: (err as Error).message,
      color: 'error'
    })
  }
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
      <div
        v-if="!loaded"
        class="flex w-full flex-1 flex-col items-center justify-center p-10"
      >
        <USkeleton class="h-8 w-64 mb-4" />
        <USkeleton class="h-4 w-96 mb-8" />
        <USkeleton class="h-12 w-full max-w-2xl rounded-full" />
      </div>

      <WorkspacePicker v-else-if="!activeWorkspaceId" />

      <UContainer
        v-else
        class="flex w-full flex-1 flex-col justify-center gap-8 py-10"
      >
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
            <USelect
              v-if="supportsReasoning"
              v-model="reasoningEffort"
              :items="effortItems"
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
