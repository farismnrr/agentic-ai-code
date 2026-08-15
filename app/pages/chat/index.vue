<script setup lang="ts">
import { chatModeItems, modelSupportsReasoning, reasoningEffortItems } from '../../utils/chat-options'

useSeoMeta({ title: 'New chat' })

const { loaded, activeWorkspaceId, workspaces, setActive } = useWorkspaces()
const settings = useSettings()
const { models, load: loadModels } = useModels()

if (models.value.length === 0) {
  await loadModels()
}

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

const { editorRef, syncText, handleKeydown, mentionItems } = useChatEditor(input, computed(() => settings.value.sendOnEnter))

const workspaceId = ref<string | undefined>(activeWorkspaceId.value || undefined)
watch(() => activeWorkspaceId.value, (newId) => {
  if (newId) {
    workspaceId.value = newId
  }
})
// Seeded from the saved default so the settings page actually governs this.
const modelId = ref<string | undefined>(settings.value.defaultModelId ?? undefined)
const mode = ref<'chat' | 'agent'>('chat')
const reasoningEffort = ref<'low' | 'medium' | 'high' | 'max'>('medium')
// A brand-new conversation has no id to PATCH yet, so there was previously no
// way to enable a tool (including the native terminal) before the first
// message — an agent-mode conversation started here always began with zero
// tools available, silently, which a model can (and did) paper over by
// fabricating a plausible-sounding answer instead of saying it lacks the
// capability. Collected here, then applied once the conversation exists.
const enabledToolIds = ref<string[]>([])

const modeItems = chatModeItems
const effortItems = reasoningEffortItems

const supportsReasoning = computed(() => {
  return modelSupportsReasoning(models.value.find(m => m.id === modelId.value))
})

const suggestions = [
  { label: 'Search the repo for chat components', icon: 'i-lucide-folder-search' },
  { label: 'Why is there no tailwind.config.js?', icon: 'i-lucide-help-circle' },
  { label: 'Show me an example component', icon: 'i-lucide-code' },
  { label: 'List the open pull requests', icon: 'i-simple-icons-github' }
]

const modelItems = computed(() =>
  models.value.map(model => ({ label: model.label, value: model.id, icon: 'i-lucide-box' }))
)

const workspaceItems = computed(() =>
  workspaces.value.map(w => ({ label: w.name, value: w.id }))
)

const { start } = useNewChatController(input, workspaceId, modelId, mode, reasoningEffort, enabledToolIds)
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
          :ui="{ footer: 'flex-wrap sm:flex-nowrap justify-start' }"
          @submit="start(input)"
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

          <UChatPromptSubmit />

          <template #footer>
            <USelect
              v-model="workspaceId"
              :items="workspaceItems"
              icon="i-lucide-folder"
              variant="ghost"
              size="sm"
            />
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
