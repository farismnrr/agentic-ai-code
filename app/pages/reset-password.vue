<script setup lang="ts">
import * as v from 'valibot'
import type { FormSubmitEvent } from '@nuxt/ui'

definePageMeta({ layout: 'auth' })
useSeoMeta({ title: 'Reset password' })

const route = useRoute()
const token = typeof route.query.token === 'string' ? route.query.token : ''

const schema = v.pipe(
  v.object({
    password: v.pipe(v.string(), v.minLength(8, 'At least 8 characters'), v.maxLength(128, 'Password too long')),
    confirm: v.string()
  }),
  v.forward(
    v.check(input => input.password === input.confirm, 'Passwords do not match'),
    ['confirm']
  )
)
type Schema = v.InferOutput<typeof schema>

const state = reactive({ password: '', confirm: '' })
const loading = ref(false)
const serverError = ref<string | null>(null)
const success = ref(false)

if (!token) {
  serverError.value = 'Invalid password reset link. Please request a new one.'
}

async function onSubmit(event: FormSubmitEvent<Schema>) {
  if (!token) return

  loading.value = true
  serverError.value = null
  try {
    await $fetch('/api/auth/reset', {
      method: 'POST',
      body: { token, password: event.data.password }
    })
    success.value = true
  } catch (err: unknown) {
    const fe = err as { data?: { message?: string } }
    serverError.value = fe.data?.message ?? 'Could not reset password. The link might have expired.'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div>
    <div class="mb-6 text-center">
      <h1 class="text-xl font-semibold text-highlighted">
        Set new password
      </h1>
      <p class="mt-1 text-sm text-muted">
        Enter your new password below.
      </p>
    </div>

    <UCard
      v-if="success"
      class="text-center"
    >
      <UIcon
        name="i-lucide-check-circle"
        class="w-12 h-12 mx-auto text-green-500 mb-4"
      />
      <h2 class="text-lg font-medium">
        Password updated
      </h2>
      <p class="text-sm text-muted mt-2">
        Your password has been changed successfully. You can now log in with your new password.
      </p>
      <UButton
        to="/login"
        label="Go to login"
        block
        class="mt-6"
      />
    </UCard>

    <UCard v-else>
      <UForm
        :schema="schema"
        :state="state"
        class="space-y-4"
        @submit="onSubmit"
      >
        <UAlert
          v-if="serverError"
          color="error"
          variant="soft"
          :description="serverError"
        />

        <template v-if="token">
          <UFormField
            label="New Password"
            name="password"
            required
            :ui="{ label: 'font-mono text-xs text-muted uppercase tracking-wider' }"
          >
            <UInput
              v-model="state.password"
              type="password"
              autocomplete="new-password"
              placeholder="At least 8 characters"
              autofocus
              class="w-full"
            />
          </UFormField>

          <UFormField
            label="Confirm Password"
            name="confirm"
            required
            :ui="{ label: 'font-mono text-xs text-muted uppercase tracking-wider' }"
          >
            <UInput
              v-model="state.confirm"
              type="password"
              autocomplete="new-password"
              class="w-full"
            />
          </UFormField>

          <UButton
            type="submit"
            label="Reset password"
            :loading="loading"
            block
          />
        </template>
      </UForm>
    </UCard>
  </div>
</template>
