<script setup lang="ts">
import { models } from '#shared/utils/fixtures/models'

useSeoMeta({ title: 'Model settings' })

const settings = useSettings()

const modelItems = computed(() =>
  models.map(model => ({ label: model.label, value: model.id, icon: model.icon }))
)
</script>

<template>
  <div class="space-y-4 py-4">
    <div>
      <h2 class="text-base font-semibold text-highlighted">
        Models
      </h2>
      <p class="text-sm text-muted">
        Defaults for new conversations. Existing ones keep the model they
        were started with.
      </p>
    </div>

    <UCard :ui="{ body: 'divide-y divide-default' }">
      <UFormField
        label="Default model"
        :description="models.find(m => m.id === settings.defaultModelId)?.description"
        class="flex items-start justify-between gap-4 pb-4"
      >
        <USelect
          v-model="settings.defaultModelId"
          :items="modelItems"
          :icon="models.find(m => m.id === settings.defaultModelId)?.icon"
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
