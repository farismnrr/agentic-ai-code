<script setup lang="ts">
import * as v from 'valibot'
import type { FormSubmitEvent } from '@nuxt/ui'

useSeoMeta({ title: 'Account settings' })

const settings = useSettings()
const toast = useToast()
const { conversations } = useConversations()

const schema = v.object({
  displayName: v.pipe(v.string(), v.minLength(1, 'Name is required')),
  email: v.pipe(v.string(), v.email('Enter a valid email address'))
})

type Schema = v.InferOutput<typeof schema>

const state = reactive({
  displayName: settings.value.displayName,
  email: settings.value.email
})

function onSubmit(event: FormSubmitEvent<Schema>) {
  settings.value = { ...settings.value, ...event.data }
  toast.add({ title: 'Profile saved', icon: 'i-lucide-check', color: 'success' })
}

const messageCount = computed(() =>
  conversations.value.reduce((total, c) => total + c.messages.length, 0)
)
</script>

<template>
  <div class="space-y-4 py-4">
    <div>
      <h2 class="text-base font-semibold text-highlighted">
        Account
      </h2>
      <p class="text-sm text-muted">
        Profile details and usage.
      </p>
    </div>

    <UCard>
      <UForm
        :schema="schema"
        :state="state"
        class="space-y-4"
        @submit="onSubmit"
      >
        <UFormField
          label="Display name"
          name="displayName"
          required
        >
          <UInput
            v-model="state.displayName"
            class="w-full max-w-sm"
          />
        </UFormField>

        <UFormField
          label="Email"
          name="email"
          required
        >
          <UInput
            v-model="state.email"
            type="email"
            class="w-full max-w-sm"
          />
        </UFormField>

        <UButton
          label="Save changes"
          type="submit"
        />
      </UForm>
    </UCard>

    <UCard>
      <h3 class="mb-3 text-sm font-medium text-highlighted">
        Usage this session
      </h3>

      <div class="grid gap-4 sm:grid-cols-2">
        <div>
          <p class="text-2xl font-semibold text-highlighted">
            {{ conversations.length }}
          </p>
          <p class="text-sm text-muted">
            Conversations
          </p>
        </div>
        <div>
          <p class="text-2xl font-semibold text-highlighted">
            {{ messageCount }}
          </p>
          <p class="text-sm text-muted">
            Messages
          </p>
        </div>
      </div>

      <p class="mt-4 text-xs text-dimmed">
        Counts reset on reload — nothing is persisted in this build.
      </p>
    </UCard>
  </div>
</template>
