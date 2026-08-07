<script setup lang="ts">
import * as v from 'valibot'
import type { FormSubmitEvent } from '@nuxt/ui'

definePageMeta({ layout: 'auth' })
useSeoMeta({ title: 'Sign in' })

const { login } = useAuth()
const route = useRoute()

const schema = v.object({
  email: v.pipe(v.string(), v.email('Enter a valid email address')),
  password: v.pipe(v.string(), v.minLength(8, 'At least 8 characters'))
})

type Schema = v.InferOutput<typeof schema>

const state = reactive({ email: '', password: '' })
const loading = ref(false)

async function onSubmit(event: FormSubmitEvent<Schema>) {
  loading.value = true
  // No request to make; a beat of latency keeps the button state honest
  // instead of flashing.
  await new Promise(resolve => setTimeout(resolve, 350))

  login(event.data.email)

  const redirect = typeof route.query.redirect === 'string' ? route.query.redirect : '/chat'
  await navigateTo(redirect)
}
</script>

<template>
  <div>
    <div class="mb-6 text-center">
      <h1 class="text-xl font-semibold text-highlighted">
        Sign in
      </h1>
      <p class="mt-1 text-sm text-muted">
        Any email and password will do.
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
