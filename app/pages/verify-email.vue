<script setup lang="ts">
import { clientErrorMessage } from '~/utils/client-errors'

definePageMeta({ layout: 'auth' })
useSeoMeta({ title: 'Verify email' })

const route = useRoute()
const token = typeof route.query.token === 'string' ? route.query.token : null

const state = ref<'loading' | 'success' | 'error'>('loading')
const errorMessage = ref('')

onMounted(async () => {
  if (!token) {
    state.value = 'error'
    errorMessage.value = 'Missing verification token.'
    return
  }

  try {
    await $fetch('/api/auth/verify', {
      method: 'POST',
      body: { token }
    })
    state.value = 'success'
  } catch (err: unknown) {
    state.value = 'error'
    errorMessage.value = clientErrorMessage(err, 'Verification failed. The link may have expired.')
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
        class="w-8 h-8 mx-auto animate-spin text-muted"
      />
      <h1 class="text-xl font-semibold text-highlighted">
        Verifying email...
      </h1>
      <p class="text-sm text-muted">
        Please wait while we verify your email address.
      </p>
    </div>

    <div
      v-else-if="state === 'success'"
      class="space-y-4"
    >
      <UIcon
        name="i-lucide-check-circle"
        class="w-12 h-12 mx-auto text-green-500"
      />
      <h1 class="text-xl font-semibold text-highlighted">
        Email Verified!
      </h1>
      <p class="text-sm text-muted">
        Your email address has been successfully verified.
      </p>
      <UButton
        to="/chat"
        label="Go to Chat"
        block
        class="mt-4"
      />
    </div>

    <div
      v-else-if="state === 'error'"
      class="space-y-4"
    >
      <UIcon
        name="i-lucide-x-circle"
        class="w-12 h-12 mx-auto text-red-500"
      />
      <h1 class="text-xl font-semibold text-highlighted">
        Verification Failed
      </h1>
      <p class="text-sm text-muted">
        {{ errorMessage }}
      </p>
      <UButton
        to="/login"
        label="Return to Login"
        color="neutral"
        variant="outline"
        block
        class="mt-4"
      />
    </div>
  </div>
</template>
