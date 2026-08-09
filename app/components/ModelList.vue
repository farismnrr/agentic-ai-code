<script setup lang="ts">
import type { Model } from '~/composables/useModels'

const { models, create, update, remove } = useModels()
const { providers } = useModelProviders()
const toast = useToast()

const isOpen = ref(false)
const editingModel = ref<Partial<Model>>({})

function edit(model: Model) {
  editingModel.value = { ...model }
  isOpen.value = true
}

function createNew() {
  editingModel.value = { icon: 'i-lucide-sparkles' }
  if (providers.value.length > 0) {
    editingModel.value.providerId = providers.value[0]?.id
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

const providerOptions = computed(() => providers.value.map(p => ({ label: p.name, value: p.id })))
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
            description="The exact ID used by the provider (e.g. gemini-1.5-pro)"
          >
            <UInput
              v-model="editingModel.modelId"
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
            <UInput
              v-model="editingModel.icon"
              class="w-full"
            />
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
              label="Thinking Min Tokens"
            >
              <UInput
                v-model="editingModel.thinkingMinTokens"
                type="number"
                placeholder="Default"
              />
            </UFormField>
            <UFormField
              v-if="editingModel.thinkingEnabled"
              label="Thinking Max Tokens"
            >
              <UInput
                v-model="editingModel.thinkingMaxTokens"
                type="number"
                placeholder="Default"
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
