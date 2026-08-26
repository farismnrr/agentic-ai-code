<script setup lang="ts">
import * as v from 'valibot'
import type { FormSubmitEvent } from '@nuxt/ui'

useSeoMeta({ title: 'Account settings' })

const settings = useSettings()
const toast = useToast()
const { conversations } = useConversations()

const schema = v.object({
  displayName: v.pipe(v.string(), v.minLength(1, 'Name is required'))
})

const emailSchema = v.object({
  email: v.pipe(v.string(), v.email('Enter a valid email address')),
  password: v.pipe(v.string(), v.minLength(8, 'Current password is required'))
})

type Schema = v.InferOutput<typeof schema>

const state = reactive({
  displayName: settings.value.displayName
})
const emailState = reactive({ email: '', password: '' })
const emailChangePending = ref(false)
type AuthSessionSummary = { id: string, createdAt: string, lastSeenAt: string, current: boolean }
const authSessions = ref<AuthSessionSummary[]>([])
const sessionsPending = ref(false)

async function onSubmit(event: FormSubmitEvent<Schema>) {
  await settings.update({ displayName: event.data.displayName })
  toast.add({ title: 'Profile saved', icon: 'i-lucide-check', color: 'success' })
}

async function requestEmailChange(event: FormSubmitEvent<{ email: string, password: string }>) {
  emailChangePending.value = true
  try {
    await $fetch('/api/auth/email-change', { method: 'POST', body: event.data })
    emailState.email = ''
    emailState.password = ''
    toast.add({ title: 'Confirmation sent', description: 'Check the new address to finish the change.', icon: 'i-lucide-mail-check', color: 'success' })
  } finally {
    emailChangePending.value = false
  }
}

async function loadAuthSessions() {
  sessionsPending.value = true
  try {
    authSessions.value = await $fetch<AuthSessionSummary[]>('/api/auth/sessions')
  } finally {
    sessionsPending.value = false
  }
}

async function revokeAuthSession(id: string) {
  await $fetch(`/api/auth/sessions/${id}`, { method: 'DELETE' })
  const revoked = authSessions.value.find(item => item.id === id)
  if (revoked?.current) {
    await navigateTo('/login')
    return
  }
  await loadAuthSessions()
}

async function revokeOtherAuthSessions() {
  await $fetch('/api/auth/sessions/revoke-others', { method: 'POST' })
  await loadAuthSessions()
}

onMounted(() => {
  void loadAuthSessions()
})
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

        <UFormField label="Email">
          <UInput
            :model-value="settings.email"
            type="email"
            class="w-full max-w-sm"
            disabled
          />
          <template #help>
            Changing the sign-in email requires recent authentication and confirmation at the new address.
          </template>
        </UFormField>

        <UButton
          label="Save changes"
          type="submit"
        />
      </UForm>
    </UCard>

    <UCard>
      <div class="flex items-start justify-between gap-4">
        <div>
          <h3 class="text-sm font-medium text-highlighted">
            Active sessions
          </h3>
          <p class="mt-1 text-sm text-muted">
            Revoke browser sessions you no longer recognize.
          </p>
        </div>
        <UButton
          label="Sign out others"
          color="neutral"
          variant="outline"
          :disabled="sessionsPending || authSessions.length < 2"
          @click="revokeOtherAuthSessions"
        />
      </div>
      <div
        v-if="sessionsPending"
        class="mt-4 text-sm text-muted"
      >
        Loading sessions...
      </div>
      <div
        v-else
        class="mt-4 divide-y divide-default"
      >
        <div
          v-for="authSession in authSessions"
          :key="authSession.id"
          class="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0"
        >
          <div class="min-w-0 text-sm">
            <p class="font-medium text-highlighted">
              {{ authSession.current ? 'This browser' : 'Browser session' }}
            </p>
            <p class="text-muted">
              Last active {{ new Date(authSession.lastSeenAt).toLocaleString() }}
            </p>
          </div>
          <UButton
            label="Revoke"
            color="error"
            variant="ghost"
            :disabled="authSession.current"
            @click="revokeAuthSession(authSession.id)"
          />
        </div>
      </div>
    </UCard>

    <UCard>
      <h3 class="mb-3 text-sm font-medium text-highlighted">
        Change email address
      </h3>
      <UForm
        :schema="emailSchema"
        :state="emailState"
        class="space-y-4"
        @submit="requestEmailChange"
      >
        <UFormField
          label="New email"
          name="email"
          required
        >
          <UInput
            v-model="emailState.email"
            type="email"
            autocomplete="email"
            class="w-full max-w-sm"
          />
        </UFormField>
        <UFormField
          label="Current password"
          name="password"
          required
        >
          <UInput
            v-model="emailState.password"
            type="password"
            autocomplete="current-password"
            class="w-full max-w-sm"
          />
        </UFormField>
        <UButton
          label="Send confirmation"
          type="submit"
          :loading="emailChangePending"
        />
      </UForm>
    </UCard>

    <UCard>
      <h3 class="mb-3 text-sm font-medium text-highlighted">
        Usage
      </h3>

      <div class="grid gap-4">
        <div>
          <p class="text-2xl font-semibold text-highlighted">
            {{ conversations.length }}
          </p>
          <p class="text-sm text-muted">
            Conversations
          </p>
        </div>
      </div>

      <p class="mt-4 text-xs text-dimmed">
        Your data is securely persisted to your account.
      </p>
    </UCard>
  </div>
</template>
