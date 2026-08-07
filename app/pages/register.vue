<script setup lang="ts">
import * as v from 'valibot'
import type { FormSubmitEvent } from '@nuxt/ui'

definePageMeta({ layout: 'auth' })
useSeoMeta({ title: 'Create account' })

const { register } = useAuth()
const route = useRoute()

const schema = v.pipe(
  v.object({
    name: v.pipe(v.string(), v.minLength(1, 'Name is required'), v.maxLength(100, 'Name too long')),
    email: v.pipe(v.string(), v.email('Enter a valid email address')),
    password: v.pipe(v.string(), v.minLength(8, 'At least 8 characters'), v.maxLength(128, 'Password too long')),
    confirm: v.string()
  }),
  // Cross-field, so it can't be a per-field rule — same shape as the
  // conditional transport validation in settings/mcp.vue.
  v.forward(
    v.check(input => input.password === input.confirm, 'Passwords do not match'),
    ['confirm']
  )
)

type Schema = v.InferOutput<typeof schema>

const state = reactive({ name: '', email: '', password: '', confirm: '' })
const loading = ref(false)
const serverError = ref<string | null>(null)

async function onSubmit(event: FormSubmitEvent<Schema>) {
  loading.value = true
  serverError.value = null
  try {
    await register(event.data.name, event.data.email, event.data.password, event.data.confirm)
    await navigateTo('/chat')
  } catch (err: unknown) {
    const fe = err as { data?: { message?: string }, statusCode?: number }
    if (fe?.statusCode === 429) {
      serverError.value = fe.data?.message ?? 'Too many attempts. Try again later.'
    } else {
      serverError.value = 'Could not create account. Please try again.'
    }
  } finally {
    loading.value = false
  }
}

if (route.query.error) {
  serverError.value = String(route.query.error)
}
</script>

<template>
  <div>
    <div class="mb-6 text-center">
      <h1 class="text-xl font-semibold text-highlighted">
        Create account
      </h1>
      <p class="mt-1 text-sm text-muted">
        Your data is stored securely.
      </p>
    </div>

    <UCard>
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

        <div class="space-y-3">
          <UButton
            to="/api/auth/google"
            external
            color="neutral"
            variant="outline"
            block
            icon="i-simple-icons-google"
            label="Sign up with Google"
          />
          <UButton
            to="/api/auth/github"
            external
            color="neutral"
            variant="outline"
            block
            icon="i-simple-icons-github"
            label="Sign up with GitHub"
          />
        </div>

        <USeparator label="or" />

        <UFormField
          label="Name"
          name="name"
          required
          :ui="{ label: 'font-mono text-xs text-muted uppercase tracking-wider' }"
        >
          <UInput
            v-model="state.name"
            autocomplete="name"
            placeholder="Faris"
            autofocus
            class="w-full"
          />
        </UFormField>

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
            autocomplete="new-password"
            placeholder="At least 8 characters"
            class="w-full"
          />
        </UFormField>

        <UFormField
          label="Confirm password"
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
          label="Create account"
          :loading="loading"
          block
        />
      </UForm>
    </UCard>

    <p class="mt-4 text-center text-sm text-muted">
      Already have one?
      <ULink to="/login">
        Sign in
      </ULink>
    </p>
  </div>
</template>
