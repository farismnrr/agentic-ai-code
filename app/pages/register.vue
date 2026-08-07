<script setup lang="ts">
import * as v from 'valibot'
import type { FormSubmitEvent } from '@nuxt/ui'

definePageMeta({ layout: 'auth' })
useSeoMeta({ title: 'Create account' })

const { register } = useAuth()

const schema = v.pipe(
  v.object({
    name: v.pipe(v.string(), v.minLength(1, 'Name is required')),
    email: v.pipe(v.string(), v.email('Enter a valid email address')),
    password: v.pipe(v.string(), v.minLength(8, 'At least 8 characters')),
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

async function onSubmit(event: FormSubmitEvent<Schema>) {
  loading.value = true
  await new Promise(resolve => setTimeout(resolve, 350))

  register(event.data.email, event.data.name)
  await navigateTo('/chat')
}
</script>

<template>
  <div>
    <div class="mb-6 text-center">
      <h1 class="text-xl font-semibold text-highlighted">
        Create account
      </h1>
      <p class="mt-1 text-sm text-muted">
        Nothing is stored anywhere. It's a prototype.
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
          label="Name"
          name="name"
          required
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
