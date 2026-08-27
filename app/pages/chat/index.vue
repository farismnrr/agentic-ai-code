<script setup lang="ts">
import { chatModeItems, modelSupportsReasoning, reasoningEffortItems } from '../../utils/chat-options'
import { resolveNewChatModelId } from '../../utils/chat-model-selection'
import type { Conversation } from '#shared/types/chat'

useSeoMeta({ title: 'New chat' })

const toast = useToast()
const { loaded, activeWorkspaceId, workspaces, setActive } = useWorkspaces()
const { conversations } = useConversations()
const settings = useSettings()
const { models, load: loadModels } = useModels()
const { capabilities, load: loadCapabilities } = useChatCapabilities()

if (models.value.length === 0) {
  await loadModels()
}
await loadCapabilities().catch(() => undefined)

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
  if (newId) workspaceId.value = newId
})

const modelId = ref<string | undefined>()
function resolveWorkspaceModel(currentWorkspaceId: string | undefined) {
  return resolveNewChatModelId({
    workspaceId: currentWorkspaceId,
    conversations: conversations.value,
    validModelIds: models.value.map(model => model.id),
    defaultModelId: settings.value.defaultModelId
  })
}

// A workspace owns the "last used model" preference implicitly through its
// newest conversation. Re-resolve on workspace changes, while preserving an
// explicit current selection across unrelated reactive updates.
watch(
  [workspaceId, conversations, models, () => settings.value.defaultModelId],
  ([currentWorkspaceId], previous) => {
    const previousWorkspaceId = previous?.[0]
    const currentModelStillValid = Boolean(modelId.value && models.value.some(model => model.id === modelId.value))
    if (currentWorkspaceId !== previousWorkspaceId || !currentModelStillValid) {
      modelId.value = resolveWorkspaceModel(currentWorkspaceId)
    }
  },
  { immediate: true }
)

const mode = ref<'chat' | 'agent'>('chat')
const reasoningEffort = ref<'low' | 'medium' | 'high' | 'max'>('medium')
const permissionMode = ref<Conversation['permissionMode']>('manual')

const modeItems = chatModeItems
const effortItems = reasoningEffortItems
const agentAvailable = computed(() => capabilities.value.terminal.available)

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

const { start } = useNewChatController(input, workspaceId, modelId, mode, reasoningEffort, permissionMode)

function startChat(text: string) {
  if (!modelId.value) {
    toast.add({
      title: 'Choose a model first',
      description: 'Select one of your configured models before starting this chat.',
      icon: 'i-lucide-box',
      color: 'neutral'
    })
    return
  }
  void start(text)
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
        class="flex w-full flex-1 flex-col justify-center gap-6 py-10"
      >
        <div class="text-center">
          <h1 class="text-2xl font-semibold text-highlighted sm:text-3xl">
            What are we building?
          </h1>
          <p class="mt-2 text-muted">
            Ask anything. Tool access follows your mode and MCP settings.
          </p>
        </div>

        <UAlert
          v-if="!modelId"
          icon="i-lucide-box"
          title="Choose a model to get started"
          description="Pick a configured model below. AI Code remembers the last model used in each workspace."
          color="neutral"
          variant="subtle"
          class="mx-auto w-full max-w-2xl"
        >
          <template #actions>
            <UButton
              to="/settings/models"
              label="Manage models"
              color="neutral"
              variant="outline"
              size="xs"
            />
          </template>
        </UAlert>

        <UChatPrompt
          v-model="input"
          :ui="{ footer: 'flex-wrap sm:flex-nowrap justify-start' }"
          @submit="startChat(input)"
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

          <UChatPromptSubmit :disabled="!modelId" />

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
              v-model:permission-mode="permissionMode"
              :model-items="modelItems"
              :mode-items="modeItems"
              :effort-items="effortItems"
              :supports-reasoning="supportsReasoning"
              :agent-available="agentAvailable"
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
            :disabled="!modelId"
            @click="startChat(suggestion.label)"
          />
        </div>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
