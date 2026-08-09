<script setup lang="ts">
import type { ModelProvider } from '~/composables/useModelProviders'

const { providers, types, create, update, remove } = useModelProviders()
const toast = useToast()

type EditingProvider = Omit<Partial<ModelProvider>, 'baseUrl'> & { apiKey?: string, baseUrl?: string }

const isOpen = ref(false)
const editingProvider = ref<EditingProvider>({})

function edit(provider: ModelProvider) {
  editingProvider.value = { ...provider, baseUrl: provider.baseUrl ?? undefined }
  isOpen.value = true
}

function createNew() {
  editingProvider.value = { type: '9router', enabled: true }
  isOpen.value = true
}

async function save() {
  try {
    if (editingProvider.value.id) {
      await update(editingProvider.value.id, editingProvider.value)
      toast.add({ title: 'Provider updated' })
    } else {
      await create(editingProvider.value as { apiKey: string })
      toast.add({ title: 'Provider created' })
    }
    isOpen.value = false
  } catch (err: unknown) {
    const error = err as Error
    toast.add({ title: 'Error', description: error.message, color: 'error' })
  }
}

async function removeProvider(id: string) {
  if (!confirm('Are you sure?')) return
  await remove(id)
  toast.add({ title: 'Provider removed' })
}
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <h3 class="text-sm font-semibold text-highlighted">
        Providers
      </h3>
      <UButton
        label="Add Provider"
        size="xs"
        color="primary"
        @click="createNew"
      />
    </div>
    <div
      v-for="p in providers"
      :key="p.id"
      class="flex items-center justify-between p-4 border border-default rounded-md"
    >
      <div>
        <div class="font-medium text-highlighted">
          {{ p.name }}
        </div>
        <div class="text-xs text-muted">
          {{ p.type }}
        </div>
      </div>
      <div class="flex gap-2">
        <UButton
          icon="i-lucide-pencil"
          size="xs"
          variant="ghost"
          @click="edit(p)"
        />
        <UButton
          icon="i-lucide-trash"
          size="xs"
          variant="ghost"
          color="error"
          @click="removeProvider(p.id)"
        />
      </div>
    </div>

    <UModal
      v-model:open="isOpen"
      title="Provider Settings"
    >
      <template #body>
        <div class="space-y-4">
          <UFormField label="Name">
            <UInput
              v-model="editingProvider.name"
              class="w-full"
            />
          </UFormField>
          <UFormField label="Type">
            <USelect
              v-model="editingProvider.type"
              :items="types"
              class="w-full"
            />
          </UFormField>
          <UFormField
            v-if="editingProvider.type === '9router'"
            label="Base URL"
          >
            <UInput
              v-model="editingProvider.baseUrl"
              class="w-full"
            />
          </UFormField>
          <UFormField label="API Key">
            <UInput
              v-model="editingProvider.apiKey"
              type="password"
              class="w-full"
              placeholder="Enter API key"
            />
          </UFormField>
          <UFormField label="Enabled">
            <USwitch v-model="editingProvider.enabled" />
          </UFormField>
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
