<script setup lang="ts">
import type { Model } from '~/composables/useModels'

const { models, create, update, remove } = useModels()
const { providers, listModels } = useModelProviders()
const toast = useToast()

defineProps<{
  providerOptions: { label: string, value: string }[]
  iconOptions: { label: string, value: string, icon: string }[]
}>()

type NullableNumericFields = 'contextWindow' | 'maxOutputTokens' | 'thinkingMinTokens' | 'thinkingMaxTokens'
type EditingModel = Omit<Partial<Model>, NullableNumericFields | 'thinkingEnabled'> & {
  [K in NullableNumericFields]?: number
} & { thinkingEnabled?: boolean }

const isOpen = ref(false)
const editingModel = ref<EditingModel>({})

// Same low/medium/high/max shape as the per-conversation reasoning-effort
// picker (see app/pages/chat/index.vue's effortItems) — raw token inputs
// forced users to know provider-specific numbers off the top of their head.
// Ranges follow the thinking-budget conventions most providers document
// (Anthropic's extended thinking budget, Gemini's thinkingBudget): low for
// quick/cheap reasoning, max for the hardest multi-step problems.
const THINKING_BUDGET_PRESETS = {
  low: { min: 1024, max: 4096 },
  medium: { min: 4096, max: 8192 },
  high: { min: 8192, max: 16384 },
  max: { min: 16384, max: 32768 }
} as const
type ThinkingBudgetPreset = keyof typeof THINKING_BUDGET_PRESETS

const thinkingBudgetItems = [
  { label: 'Low', value: 'low' },
  { label: 'Medium', value: 'medium' },
  { label: 'High', value: 'high' },
  { label: 'Max', value: 'max' }
]

function presetFromTokens(min?: number, max?: number): ThinkingBudgetPreset {
  const match = (Object.entries(THINKING_BUDGET_PRESETS) as [ThinkingBudgetPreset, { min: number, max: number }][])
    .find(([, range]) => range.min === min && range.max === max)
  return match?.[0] ?? 'medium'
}

const thinkingBudget = computed<ThinkingBudgetPreset>({
  get: () => presetFromTokens(editingModel.value.thinkingMinTokens, editingModel.value.thinkingMaxTokens),
  set: (preset: ThinkingBudgetPreset) => {
    const range = THINKING_BUDGET_PRESETS[preset]
    editingModel.value.thinkingMinTokens = range.min
    editingModel.value.thinkingMaxTokens = range.max
  }
})

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

// Checking "Thinking Enabled" with no budget picked yet shouldn't submit
// undefined min/max while the select visually shows "Medium" (its fallback
// display value) — pin the fields to that same default the moment thinking
// turns on.
watch(() => editingModel.value.thinkingEnabled, (enabled) => {
  if (enabled && editingModel.value.thinkingMinTokens === undefined && editingModel.value.thinkingMaxTokens === undefined) {
    thinkingBudget.value = 'medium'
  }
})

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
            <UFormField label="Thinking Enabled">
              <UCheckbox v-model="editingModel.thinkingEnabled" />
            </UFormField>
            <UFormField
              v-if="editingModel.thinkingEnabled"
              label="Thinking Budget"
              description="How much of the model's reasoning budget to use — higher tiers think longer on hard problems but cost more."
            >
              <USelect
                v-model="thinkingBudget"
                :items="thinkingBudgetItems"
                class="w-full"
              />
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
