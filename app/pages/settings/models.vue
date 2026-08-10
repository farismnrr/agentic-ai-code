<script setup lang="ts">
import type { ModelProviderTypeOption } from '~/composables/useModelProviders'

useSeoMeta({ title: 'Model settings' })

const settings = useSettings()

const { data, pending, error, refresh } = useLazyFetch('/api/settings/models-config')

const { models } = useModels()
const { providers, types } = useModelProviders()

watch(data, (newData) => {
  if (newData) {
    models.value = newData.models
    providers.value = newData.providers
    types.value = newData.providerTypes as ModelProviderTypeOption[]
  }
}, { immediate: true })

// Derived from the live `models` state so a model added/removed below
// updates this dropdown immediately, without needing the page reloaded.
const modelItems = computed(() =>
  models.value.map(m => ({ label: m.label, value: m.id, icon: 'i-lucide-box' }))
)

const defaultModelId = computed({
  get: () => settings.value.defaultModelId ?? undefined,
  set: (value: string | undefined) => { settings.value.defaultModelId = value ?? null }
})
</script>

<template>
  <div class="space-y-8 py-4">
    <DataLoadError
      v-if="error"
      title="Couldn't load model configuration"
      description="Failed to load model providers and settings."
      @retry="refresh()"
    />

    <template v-else-if="pending">
      <div class="space-y-4">
        <USkeleton class="h-6 w-36 rounded" />
        <USkeleton class="h-4 w-64 rounded" />
        <div class="space-y-3 pt-2">
          <USkeleton class="h-16 w-full rounded-lg" />
          <USkeleton class="h-16 w-full rounded-lg" />
        </div>
      </div>
      <div class="space-y-4 pt-4">
        <USkeleton class="h-6 w-24 rounded" />
        <USkeleton class="h-4 w-56 rounded" />
        <div class="space-y-3 pt-2">
          <USkeleton class="h-16 w-full rounded-lg" />
        </div>
      </div>
    </template>

    <template v-else>
      <div>
        <h2 class="text-base font-semibold text-highlighted">
          Model Providers
        </h2>
        <p class="text-sm text-muted mb-4">
          Configure your model providers (e.g. 9Router, GCP Agent Platform).
        </p>
        <ProviderList />
      </div>

      <div>
        <h2 class="text-base font-semibold text-highlighted">
          Models
        </h2>
        <p class="text-sm text-muted mb-4">
          Add and configure specific models from your providers.
        </p>
        <ModelList :icon-options="data?.iconOptions || []" />
      </div>

      <div>
        <h2 class="text-base font-semibold text-highlighted">
          Default Settings
        </h2>
        <p class="text-sm text-muted mb-4">
          Defaults for new conversations. Existing ones keep the model they
          were started with.
        </p>
      </div>

      <UCard :ui="{ body: 'divide-y divide-default' }">
        <UFormField
          label="Default model"
          :description="models?.find(m => m.id === settings.defaultModelId)?.description"
          class="flex items-start justify-between gap-4 pb-4"
        >
          <USelect
            v-model="defaultModelId"
            :items="modelItems"
            icon="i-lucide-box"
            class="w-56"
          />
        </UFormField>

        <UFormField
          label="Temperature"
          :description="`${settings.temperature.toFixed(1)} — lower is more focused, higher is more varied.`"
          class="py-4"
        >
          <USlider
            v-model="settings.temperature"
            :min="0"
            :max="2"
            :step="0.1"
            class="mt-2"
          />
        </UFormField>

        <UFormField
          label="Custom instructions"
          description="Prepended to every conversation as a system prompt."
          class="pt-4"
        >
          <UTextarea
            v-model="settings.systemPrompt"
            :rows="5"
            autoresize
            placeholder="Reply concisely. Prefer TypeScript examples."
            class="mt-2 w-full"
          />
        </UFormField>
      </UCard>
    </template>
  </div>
</template>
