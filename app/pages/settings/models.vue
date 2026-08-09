<script setup lang="ts">
import type { ModelProviderTypeOption } from '~/composables/useModelProviders'

useSeoMeta({ title: 'Model settings' })

const settings = useSettings()

const { data } = await useFetch('/api/settings/models-config')

const { models } = useModels()
const { providers, types } = useModelProviders()

if (data.value) {
  models.value = data.value.models
  providers.value = data.value.providers
  types.value = data.value.providerTypes as ModelProviderTypeOption[]
}

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
  </div>
</template>
