<script setup lang="ts">
import type { Model } from '~/composables/useModels'

const { models, create, update, remove } = useModels()
const { providers, listModels } = useModelProviders()
const toast = useToast()

defineProps<{
  iconOptions: { label: string, value: string, icon: string }[]
}>()

// Derived from the live `providers` state (not a snapshot prop) so a
// provider added in ProviderList.vue shows up here immediately — the two
// components share the same `useModelProviders()` state, so this updates
// the moment ProviderList's create()/remove() mutates it, no page reload.
const providerOptions = computed(() => providers.value.map(p => ({ label: p.name, value: p.id })))

type NullableNumericFields = 'contextWindow' | 'maxOutputTokens' | 'thinkingMinTokens' | 'thinkingMaxTokens'
type EditingModel = Omit<Partial<Model>, NullableNumericFields | 'thinkingEnabled'> & {
  [K in NullableNumericFields]?: number
} & { thinkingEnabled?: boolean }

const isOpen = ref(false)
const editingModel = ref<EditingModel>({})

// Live model IDs fetched from the selected provider's own API, keyed by
// providerId so switching providers in the form doesn't require a refetch
// if the user flips back and forth. `create-item` on the USelectMenu still
// lets the user type a model ID by hand when the live list is empty/errored
// (e.g. the provider's key was just added and hasn't been saved yet).
const modelIdOptionsByProvider = ref<Record<string, { label: string, value: string }[]>>({})
const modelIdOptionsLoading = ref(false)

async function loadModelIdOptions(providerId: string | undefined) {
  if (!providerId || modelIdOptionsByProvider.value[providerId]) return
  modelIdOptionsLoading.value = true
  try {
    modelIdOptionsByProvider.value[providerId] = await listModels(providerId)
  } catch (err: unknown) {
    const error = err as Error
    toast.add({ title: 'Could not load model list', description: error.message, color: 'warning' })
  } finally {
    modelIdOptionsLoading.value = false
  }
}

const modelIdOptions = computed(() => {
  const providerId = editingModel.value.providerId
  return providerId ? (modelIdOptionsByProvider.value[providerId] ?? []) : []
})

watch(() => editingModel.value.providerId, providerId => loadModelIdOptions(providerId))

function edit(model: Model) {
  editingModel.value = {
    ...model,
    contextWindow: model.contextWindow ?? undefined,
    maxOutputTokens: model.maxOutputTokens ?? undefined,
    thinkingEnabled: model.thinkingEnabled ?? undefined,
    thinkingMinTokens: model.thinkingMinTokens ?? undefined,
    thinkingMaxTokens: model.thinkingMaxTokens ?? undefined
  }
  isOpen.value = true
  loadModelIdOptions(model.providerId)
}

function createNew() {
  editingModel.value = { icon: 'i-lucide-sparkles' }
  if (providers.value.length > 0) {
    editingModel.value.providerId = providers.value[0]?.id
    loadModelIdOptions(editingModel.value.providerId)
  }
  isOpen.value = true
}

async function save() {
  try {
    if (editingModel.value.id) {
      await update(editingModel.value.id, editingModel.value)
      toast.add({ title: 'Model updated' })
    } else {
      await create(editingModel.value)
      toast.add({ title: 'Model created' })
    }
    isOpen.value = false
  } catch (err: unknown) {
    const error = err as Error
    toast.add({ title: 'Error', description: error.message, color: 'error' })
  }
}

async function removeModel(id: string) {
  if (!confirm('Are you sure?')) return
  await remove(id)
  toast.add({ title: 'Model removed' })
}
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <h3 class="text-sm font-semibold text-highlighted">
        Models
      </h3>
      <UButton
        label="Add Model"
        size="xs"
        color="primary"
        :disabled="providers.length === 0"
        @click="createNew"
      />
    </div>
    <div
      v-for="m in models"
      :key="m.id"
      class="flex items-center justify-between p-4 border border-default rounded-md"
    >
      <div>
        <div class="font-medium text-highlighted flex items-center gap-2">
          <UIcon :name="m.icon || 'i-lucide-sparkles'" />
          {{ m.label }}
        </div>
        <div class="text-xs text-muted">
          {{ m.modelId }}
        </div>
      </div>
      <div class="flex gap-2">
        <UButton
          icon="i-lucide-pencil"
          size="xs"
          variant="ghost"
          @click="edit(m)"
        />
        <UButton
          icon="i-lucide-trash"
          size="xs"
          variant="ghost"
          color="error"
          @click="removeModel(m.id)"
        />
      </div>
    </div>

    <UModal
      v-model:open="isOpen"
      title="Model Settings"
    >
      <template #body>
        <div class="space-y-4 max-h-[60vh] overflow-y-auto px-1">
          <UFormField label="Provider">
            <USelect
              v-model="editingModel.providerId"
              :items="providerOptions"
              class="w-full"
            />
          </UFormField>
          <UFormField
            label="Model ID"
            :description="modelIdOptionsLoading ? 'Loading models from provider…' : 'Fetched live from the provider — type to search, or enter one by hand'"
          >
            <USelectMenu
              v-model="editingModel.modelId"
              :items="modelIdOptions"
              :loading="modelIdOptionsLoading"
              value-key="value"
              create-item
              class="w-full"
            />
          </UFormField>
          <UFormField label="Label">
            <UInput
              v-model="editingModel.label"
              class="w-full"
            />
          </UFormField>
          <UFormField label="Description">
            <UInput
              v-model="editingModel.description"
              class="w-full"
            />
          </UFormField>
          <UFormField label="Icon">
            <USelectMenu
              v-model="editingModel.icon"
              :items="iconOptions"
              value-key="value"
              create-item
              class="w-full"
            >
              <template #item="{ item }">
                <UIcon
                  :name="item.icon || item.value || item"
                  class="w-5 h-5 flex-shrink-0"
                />
                <span class="truncate">{{ item.label || item }}</span>
              </template>
              <template #leading>
                <UIcon
                  v-if="editingModel.icon"
                  :name="editingModel.icon"
                  class="w-5 h-5 flex-shrink-0"
                />
              </template>
            </USelectMenu>
          </UFormField>

          <UCard :ui="{ body: 'space-y-4' }">
            <template #header>
              Overrides
            </template>
            <UFormField label="Context Window">
              <UInput
                v-model="editingModel.contextWindow"
                type="number"
                placeholder="Default"
              />
            </UFormField>
            <UFormField label="Max Output Tokens">
              <UInput
                v-model="editingModel.maxOutputTokens"
                type="number"
                placeholder="Default"
              />
            </UFormField>
            <UFormField
              label="Thinking Enabled"
              description="Reasoning effort itself is picked per-conversation in the chat picker — this just controls whether the model supports it at all."
            >
              <UCheckbox v-model="editingModel.thinkingEnabled" />
            </UFormField>
          </UCard>
        </div>
      </template>
      <template #footer>
        <div class="flex justify-end gap-2">
          <UButton
            label="Cancel"
            variant="ghost"
            color="neutral"
            @click="isOpen = false"
          />
          <UButton
            label="Save"
            @click="save"
          />
        </div>
      </template>
    </UModal>
  </div>
</template>
