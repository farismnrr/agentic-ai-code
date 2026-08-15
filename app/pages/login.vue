<script setup lang="ts">
import type { FormSubmitEvent } from '@nuxt/ui'
import { clientErrorMessage } from '~/utils/client-errors'

definePageMeta({ layout: 'auth' })
useSeoMeta({ title: 'Sign in' })

const { login } = useAuth()
const route = useRoute()

const state = reactive({ email: '', password: '' })
const loading = ref(false)
const serverError = ref<string | null>(null)

const toast = useToast()

async function onSubmit(event: FormSubmitEvent<typeof state>) {
  loading.value = true
  serverError.value = null
  try {
    await login(event.data.email, event.data.password)
    const redirect = typeof route.query.redirect === 'string' ? route.query.redirect : '/chat'
    await navigateTo(redirect)
  } catch (err: unknown) {
    const statusCode = (err as { statusCode?: number })?.statusCode
    if (statusCode === 429) {
      serverError.value = clientErrorMessage(err, 'Too many attempts. Try again later.')
    } else {
      serverError.value = clientErrorMessage(err, 'Invalid email or password.')
    }
    toast.add({
      title: 'Sign in failed',
      description: serverError.value,
      color: 'error'
    })
  } finally {
    loading.value = false
  }
}

// Display errors passed via URL (e.g., from OAuth redirect)
if (route.query.error) {
  serverError.value = 'Sign in could not be completed. Please try again.'
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

        <div class="space-y-3">
          <UButton
            to="/api/auth/google"
            external
            color="neutral"
            variant="outline"
            block
            icon="i-simple-icons-google"
            label="Continue with Google"
          />
          <UButton
            to="/api/auth/github"
            external
            color="neutral"
            variant="outline"
            block
            icon="i-simple-icons-github"
            label="Continue with GitHub"
          />
        </div>

        <USeparator label="or" />

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
          <div class="flex flex-col space-y-2">
            <UInput
              v-model="state.password"
              type="password"
              autocomplete="current-password"
              placeholder="••••••••"
              class="w-full"
            />
            <div class="text-right">
              <ULink
                to="/forgot-password"
                class="text-xs text-muted hover:text-primary transition-colors"
              >
                Forgot password?
              </ULink>
            </div>
          </div>
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
