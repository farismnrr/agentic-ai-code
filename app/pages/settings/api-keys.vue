<script setup lang="ts">
import * as v from 'valibot'
import type { FormSubmitEvent } from '@nuxt/ui'

useSeoMeta({ title: 'API Keys' })

const toast = useToast()
const { data: keys, refresh } = await useFetch('/api/api-keys', {
  default: () => []
})

const addOpen = ref(false)
const rawKeyResult = ref<string | null>(null)

const schema = v.object({
  name: v.pipe(v.string(), v.minLength(1, 'Name is required'))
})
type Schema = v.InferOutput<typeof schema>

const state = reactive<{ name: string }>({ name: '' })

function copyRawKey() {
  if (rawKeyResult.value) {
    navigator.clipboard.writeText(rawKeyResult.value)
    toast.add({ title: 'Copied!' })
  }
}

async function onSubmit(event: FormSubmitEvent<Schema>) {
  try {
    const result = await $fetch('/api/api-keys', {
      method: 'POST',
      body: { name: event.data.name }
    })
    rawKeyResult.value = result.rawKey
    toast.add({ title: `Added ${event.data.name}`, icon: 'i-lucide-check', color: 'success' })
    addOpen.value = false
    state.name = ''
    refresh()
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : 'Unknown error'
    toast.add({ title: 'Failed to create key', description: message, icon: 'i-lucide-alert-triangle', color: 'error' })
  }
}

async function removeKey(id: string, name: string) {
  try {
    await $fetch(`/api/api-keys/${id}`, { method: 'DELETE' })
    toast.add({ title: `Revoked ${name}`, icon: 'i-lucide-trash-2', color: 'neutral' })
    refresh()
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : 'Unknown error'
    toast.add({ title: 'Failed to revoke key', description: message, icon: 'i-lucide-alert-triangle', color: 'error' })
  }
}
</script>

<template>
  <div class="space-y-4 py-4">
    <div class="flex flex-col sm:flex-row sm:items-start justify-between gap-4">
      <div>
        <h2 class="text-base font-semibold text-highlighted">
          API Keys
        </h2>
        <p class="text-sm text-muted">
          Keys to authenticate external MCP clients.
        </p>
      </div>

      <UButton
        label="Create key"
        icon="i-lucide-plus"
        @click="addOpen = true"
      />
    </div>

    <UAlert
      v-if="rawKeyResult"
      title="Save your new API Key"
      description="This is the only time it will be shown. Copy it now."
      icon="i-lucide-alert-circle"
      color="warning"
      variant="subtle"
      class="mb-4"
      :actions="[{ label: 'Copy to clipboard', onClick: copyRawKey }, { label: 'Dismiss', color: 'neutral', onClick: () => rawKeyResult = null }]"
    >
      <template #description>
        <p class="mb-2">
          This is the only time it will be shown. Copy it now.
        </p>
        <code class="block break-all rounded bg-elevated p-2 text-highlighted">{{ rawKeyResult }}</code>
      </template>
    </UAlert>

    <UCard
      v-if="!keys.length"
      class="border-dashed"
      :ui="{ body: 'flex flex-col items-center justify-center py-12 text-center' }"
    >
      <div class="mb-4 flex size-10 items-center justify-center rounded-full bg-elevated">
        <UIcon
          name="i-lucide-key"
          class="size-5 text-muted"
        />
      </div>
      <h3 class="mb-1 text-sm font-medium text-highlighted">
        No API keys
      </h3>
      <p class="mb-4 text-sm text-muted">
        Create a key to connect an external MCP client.
      </p>
      <UButton
        label="Create key"
        icon="i-lucide-plus"
        color="neutral"
        variant="outline"
        @click="addOpen = true"
      />
    </UCard>

    <UCard
      v-for="apiKey in keys"
      :key="apiKey.id"
      :ui="{ body: 'space-y-3' }"
    >
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <p class="font-medium text-highlighted">
              {{ apiKey.name }}
            </p>
          </div>
          <code class="text-xs break-all text-dimmed">{{ apiKey.keyPrefix }}••••••••••••••••</code>
          <p class="text-xs text-muted mt-1">
            Created: {{ new Date(apiKey.createdAt).toLocaleDateString() }}
            <span v-if="apiKey.lastUsedAt">&bull; Last used: {{ new Date(apiKey.lastUsedAt).toLocaleDateString() }}</span>
          </p>
        </div>

        <div class="flex items-center gap-2">
          <UButton
            icon="i-lucide-trash-2"
            color="error"
            variant="ghost"
            size="sm"
            :aria-label="`Revoke ${apiKey.name}`"
            @click="removeKey(apiKey.id, apiKey.name)"
          />
        </div>
      </div>
    </UCard>

    <UModal
      v-model:open="addOpen"
      title="Create API Key"
    >
      <template #body>
        <UForm
          id="create-key"
          :schema="schema"
          :state="state"
          class="space-y-4"
          @submit="onSubmit"
        >
          <UFormField
            label="Name"
            name="name"
            required
          >
            <UInput
              v-model="state.name"
              placeholder="Claude Desktop"
              class="w-full"
            />
          </UFormField>
        </UForm>
      </template>

      <template #footer>
        <div class="flex w-full justify-end gap-2">
          <UButton
            label="Cancel"
            color="neutral"
            variant="ghost"
            @click="addOpen = false"
          />
          <UButton
            label="Create"
            type="submit"
            form="create-key"
          />
        </div>
      </template>
    </UModal>
  </div>
</template>
