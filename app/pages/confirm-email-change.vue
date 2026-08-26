<script setup lang="ts">
import { clientErrorMessage } from '~/utils/client-errors'

definePageMeta({ layout: 'auth' })
useSeoMeta({ title: 'Confirm email change' })

const state = ref<'loading' | 'success' | 'error'>('loading')
const errorMessage = ref('')

onMounted(async () => {
  const token = window.location.hash.match(/(?:^|#)token=([^&]+)/)?.[1]
  window.history.replaceState({}, '', window.location.pathname)
  if (!token) {
    state.value = 'error'
    errorMessage.value = 'Missing confirmation token.'
    return
  }
  try {
    await $fetch('/api/auth/email-change/confirm', { method: 'POST', body: { token: decodeURIComponent(token) } })
    state.value = 'success'
  } catch (error: unknown) {
    state.value = 'error'
    errorMessage.value = clientErrorMessage(error, 'The confirmation link may have expired or already been used.')
  }
})
</script>

<template>
  <div class="text-center">
    <div
      v-if="state === 'loading'"
      class="space-y-4"
    >
      <UIcon
        name="i-lucide-loader-2"
        class="mx-auto h-8 w-8 animate-spin text-muted"
      />
      <h1 class="text-xl font-semibold text-highlighted">
        Confirming email change...
      </h1>
    </div>
    <div
      v-else-if="state === 'success'"
      class="space-y-4"
    >
      <UIcon
        name="i-lucide-check-circle"
        class="mx-auto h-12 w-12 text-green-500"
      />
      <h1 class="text-xl font-semibold text-highlighted">
        Email changed
      </h1>
      <p class="text-sm text-muted">
        Your email was changed. Sign in again with the new address.
      </p>
      <UButton
        to="/login"
        label="Sign in"
        block
        class="mt-4"
      />
    </div>
    <div
      v-else
      class="space-y-4"
    >
      <UIcon
        name="i-lucide-x-circle"
        class="mx-auto h-12 w-12 text-red-500"
      />
      <h1 class="text-xl font-semibold text-highlighted">
        Confirmation failed
      </h1>
      <p class="text-sm text-muted">
        {{ errorMessage }}
      </p>
      <UButton
        to="/login"
        label="Return to login"
        color="neutral"
        variant="outline"
        block
        class="mt-4"
      />
    </div>
  </div>
</template>
