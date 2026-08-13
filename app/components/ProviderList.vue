<script setup lang="ts">
import type { ModelProvider } from '~/composables/useModelProviders'
import { providerRequiresBaseUrl } from '#shared/utils/providers'

const { providers, types, create, update, remove } = useModelProviders()
const toast = useToast()

type EditingProvider = Omit<Partial<ModelProvider>, 'baseUrl' | 'customHeaderKeys'> & { apiKey?: string, baseUrl?: string }

const isOpen = ref(false)
const editingProvider = ref<EditingProvider>({})
// Header values are secrets and the server never sends them back — a row
// carrying `existing: true` and a blank value means "keep this header's
// current value unchanged"; typing a value replaces it.
const headerRows = ref<{ key: string, value: string, existing: boolean }[]>([])
const originalHeaderKeys = ref<string[]>([])

function typeLabel(type: ModelProvider['type']) {
  return types.value.find(t => t.value === type)?.label ?? type
}

const requiresBaseUrl = computed(() => providerRequiresBaseUrl(editingProvider.value.type ?? 'openai_compatible'))

function edit(provider: ModelProvider) {
  editingProvider.value = { ...provider, baseUrl: provider.baseUrl ?? undefined }
  originalHeaderKeys.value = provider.customHeaderKeys
  headerRows.value = provider.customHeaderKeys.map(key => ({ key, value: '', existing: true }))
  isOpen.value = true
}

function createNew() {
  editingProvider.value = { type: types.value[0]?.value ?? 'openai_compatible', enabled: true }
  originalHeaderKeys.value = []
  headerRows.value = []
  isOpen.value = true
}

function addHeaderRow() {
  headerRows.value.push({ key: '', value: '', existing: false })
}

function removeHeaderRow(index: number) {
  headerRows.value.splice(index, 1)
}

/** Builds the customHeaders diff described in the PUT schema: a set value
 * replaces/adds that header, `null` deletes it, and omitted keys are left
 * untouched server-side (used because unchanged secret values are never
 * available to resend). */
function buildHeaderDiff() {
  const diff: Record<string, string | null> = {}
  const remainingKeys = new Set(headerRows.value.map(row => row.key.trim()).filter(Boolean))
  for (const key of originalHeaderKeys.value) {
    if (!remainingKeys.has(key)) diff[key] = null
  }
  for (const row of headerRows.value) {
    const key = row.key.trim()
    if (!key) continue
    if (row.value.trim() !== '') diff[key] = row.value
    // else: an existing header left blank keeps its current value untouched.
  }
  return diff
}

async function save() {
  try {
    const customHeaders = buildHeaderDiff()
    const payload = { ...editingProvider.value, customHeaders }

    if (editingProvider.value.id) {
      await update(editingProvider.value.id, payload)
      toast.add({ title: 'Provider updated' })
    } else {
      await create(payload as { apiKey: string })
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
          {{ typeLabel(p.type) }}
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
        <div class="space-y-4 max-h-[60vh] overflow-y-auto px-1">
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
            v-if="requiresBaseUrl"
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

          <UFormField
            v-if="requiresBaseUrl"
            label="Custom Headers"
            description="Extra HTTP headers sent with every request to this provider — for gateways/proxies that need something beyond the API key."
          >
            <div class="space-y-2">
              <div
                v-for="(row, index) in headerRows"
                :key="index"
                class="flex gap-2"
              >
                <UInput
                  v-model="row.key"
                  placeholder="Header name"
                  class="w-1/2"
                />
                <UInput
                  v-model="row.value"
                  type="password"
                  :placeholder="row.existing ? 'Unchanged — enter to replace' : 'Value'"
                  class="w-1/2"
                />
                <UButton
                  icon="i-lucide-x"
                  size="xs"
                  variant="ghost"
                  color="neutral"
                  @click="removeHeaderRow(index)"
                />
              </div>
              <UButton
                label="Add header"
                icon="i-lucide-plus"
                size="xs"
                variant="soft"
                @click="addHeaderRow"
              />
            </div>
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
