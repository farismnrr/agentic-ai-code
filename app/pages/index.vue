<script setup lang="ts">
// Placeholder. Phase 2 replaces this with the real landing page — hero,
// features, pricing, testimonials, FAQ.
definePageMeta({ layout: false })
useSeoMeta({ title: 'AI Code' })
</script>

<template>
  <div class="flex min-h-svh flex-col items-center justify-center gap-6 p-6 text-center">
    <div>
      <h1 class="text-3xl font-semibold text-highlighted">
        AI Code
      </h1>
      <p class="mt-2 text-muted">
        Chat with models, connected to your MCP tools.
      </p>
    </div>

    <div class="flex gap-3">
      <UButton
        to="/login"
        label="Sign in"
        size="lg"
      />
      <UButton
        to="/register"
        label="Create account"
        color="neutral"
        variant="subtle"
        size="lg"
      />
    </div>

    <p class="text-xs text-dimmed">
      Landing page lands in phase 2.
    </p>
  </div>
</template>
