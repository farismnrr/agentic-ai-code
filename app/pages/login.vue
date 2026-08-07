<script setup lang="ts">
import type { FormSubmitEvent } from '@nuxt/ui'

definePageMeta({ layout: 'auth' })
useSeoMeta({ title: 'Sign in' })

const { login } = useAuth()
const route = useRoute()

const state = reactive({ email: '', password: '' })
const loading = ref(false)
const serverError = ref<string | null>(null)

async function onSubmit(event: FormSubmitEvent<typeof state>) {
  loading.value = true
  serverError.value = null
  try {
    await login(event.data.email, event.data.password)
    const redirect = typeof route.query.redirect === 'string' ? route.query.redirect : '/chat'
    await navigateTo(redirect)
  } catch (err: unknown) {
    const fe = err as { data?: { message?: string }, statusCode?: number }
    if (fe?.statusCode === 429) {
      serverError.value = fe.data?.message ?? 'Too many attempts. Try again later.'
    } else {
      // Generic message regardless of server error — don't leak account existence.
      serverError.value = 'Invalid email or password.'
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
        Sign in
      </h1>
      <p class="mt-1 text-sm text-muted">
        Sign in to your account.
      </p>
    </div>

    <UCard>
      <UForm
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

        <UFormField
          label="Password"
          name="password"
          required
          :ui="{ label: 'font-mono text-xs text-muted uppercase tracking-wider' }"
        >
          <UInput
            v-model="state.password"
            type="password"
            autocomplete="current-password"
            placeholder="••••••••"
            class="w-full"
          />
        </UFormField>

        <UButton
          type="submit"
          label="Sign in"
          :loading="loading"
          block
        />
      </UForm>
    </UCard>

    <p class="mt-4 text-center text-sm text-muted">
      No account?
      <ULink to="/register">
        Create one
      </ULink>
    </p>
  </div>
</template>
