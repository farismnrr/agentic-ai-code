<script setup lang="ts">
import * as v from 'valibot'
import type { FormSubmitEvent } from '@nuxt/ui'
import { clientErrorMessage } from '~/utils/client-errors'

definePageMeta({ layout: 'auth' })
useSeoMeta({ title: 'Forgot password' })

const schema = v.object({
  email: v.pipe(v.string(), v.email('Enter a valid email address'))
})
type Schema = v.InferOutput<typeof schema>

const state = reactive({ email: '' })
const loading = ref(false)
const serverError = ref<string | null>(null)
const success = ref(false)

async function onSubmit(event: FormSubmitEvent<Schema>) {
  loading.value = true
  serverError.value = null
  try {
    await $fetch('/api/auth/forgot', {
      method: 'POST',
      body: { email: event.data.email }
    })
    success.value = true
  } catch (err: unknown) {
    const statusCode = (err as { statusCode?: number })?.statusCode
    if (statusCode === 429) {
      serverError.value = clientErrorMessage(err, 'Too many attempts. Try again later.')
    } else {
      serverError.value = 'Could not request password reset. Please try again.'
    }
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div>
    <div class="mb-6 text-center">
      <h1 class="text-xl font-semibold text-highlighted">
        Reset password
      </h1>
      <p class="mt-1 text-sm text-muted">
        Enter your email to receive a reset link.
      </p>
    </div>

    <UCard
      v-if="success"
      class="text-center"
    >
      <UIcon
        name="i-lucide-mail"
        class="w-12 h-12 mx-auto text-highlighted mb-4"
      />
      <h2 class="text-lg font-medium">
        Check your inbox
      </h2>
      <p class="text-sm text-muted mt-2">
        If an account exists for <span class="font-semibold">{{ state.email }}</span>, a password reset link has been sent.
      </p>
      <UButton
        to="/login"
        label="Return to login"
        variant="outline"
        color="neutral"
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

        <UFormField
          label="Email"
          name="email"
          required
          :ui="{ label: 'font-mono text-xs text-muted uppercase tracking-wider' }"
        >
          <UInput
            v-model="state.email"
            type="email"
            autocomplete="email"
            placeholder="you@example.com"
            autofocus
            class="w-full"
          />
        </UFormField>

        <UButton
          type="submit"
          label="Send reset link"
          :loading="loading"
          block
        />
      </UForm>
    </UCard>

    <p
      v-if="!success"
      class="mt-4 text-center text-sm text-muted"
    >
      Remember your password?
      <ULink to="/login">Sign in</ULink>
    </p>
  </div>
</template>
