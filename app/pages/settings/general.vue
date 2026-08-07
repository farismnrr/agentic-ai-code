<script setup lang="ts">
useSeoMeta({ title: 'General settings' })

const settings = useSettings()
const { reset: resetConversations } = useConversations()
const toast = useToast()

/**
 * Nothing persists, so a demo that gets into a confusing state has no way out
 * short of a reload — which also signs nothing out, since the session is the
 * one thing that survives. This is that way out.
 */
function resetDemo() {
  resetConversations()
  toast.add({ title: 'Demo data reset', icon: 'i-lucide-rotate-ccw', color: 'success' })
}

const languages = [
  { label: 'English', value: 'en' },
  { label: 'Bahasa Indonesia', value: 'id' }
]
</script>

<template>
  <div class="space-y-4 py-4">
    <div>
      <h2 class="text-base font-semibold text-highlighted">
        General
      </h2>
      <p class="text-sm text-muted">
        Appearance and interaction preferences.
      </p>
    </div>

    <UCard :ui="{ body: 'divide-y divide-default' }">
      <UFormField
        label="Theme"
        description="Follows your system setting unless overridden."
        class="flex items-center justify-between gap-4 pb-4"
      >
        <UColorModeSelect />
      </UFormField>

      <UFormField
        label="Language"
        description="Interface language."
        class="flex items-center justify-between gap-4 py-4"
      >
        <USelect
          v-model="settings.language"
          :items="languages"
          class="w-48"
        />
      </UFormField>

      <UFormField
        label="Stream responses"
        description="Show tokens as they arrive instead of waiting for the full reply."
        class="flex items-center justify-between gap-4 py-4"
      >
        <USwitch v-model="settings.streaming" />
      </UFormField>

      <UFormField
        label="Send on Enter"
        description="Off means Enter adds a newline and ⌘+Enter sends."
        class="flex items-center justify-between gap-4 pt-4"
      >
        <USwitch v-model="settings.sendOnEnter" />
      </UFormField>
    </UCard>

    <UCard>
      <UFormField
        label="Reset demo data"
        description="Restores the seed conversations. Your session stays signed in."
        class="flex items-center justify-between gap-4"
      >
        <UButton
          label="Reset"
          icon="i-lucide-rotate-ccw"
          color="neutral"
          variant="subtle"
          @click="resetDemo"
        />
      </UFormField>
    </UCard>
  </div>
</template>
