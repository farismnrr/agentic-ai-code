<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui'

const { isAuthenticated } = useAuth()

const links: NavigationMenuItem[] = [
  { label: 'Features', to: '#features' },
  { label: 'Pricing', to: '#pricing' },
  { label: 'FAQ', to: '#faq' }
]
</script>

<template>
  <div>
    <UHeader :ui="{ center: 'hidden lg:flex' }">
      <template #left>
        <NuxtLink
          to="/"
          class="font-semibold text-highlighted"
        >
          AI Code
        </NuxtLink>
      </template>

      <UNavigationMenu :items="links" />

      <template #right>
        <UColorModeButton />

        <!-- Signed-in visitors get a way back into the app rather than a
             sign-in button that would immediately redirect them. -->
        <UButton
          v-if="isAuthenticated"
          to="/chat"
          label="Open app"
          trailing-icon="i-lucide-arrow-right"
        />
        <template v-else>
          <UButton
            to="/login"
            label="Sign in"
            color="neutral"
            variant="ghost"
            class="hidden sm:inline-flex"
          />
          <UButton
            to="/register"
            label="Get started"
          />
        </template>
      </template>

      <template #body>
        <UNavigationMenu
          :items="links"
          orientation="vertical"
        />
      </template>
    </UHeader>

    <UMain>
      <slot />
    </UMain>

    <USeparator />

    <UFooter>
      <template #left>
        <p class="text-sm text-muted">
          A prototype. No backend, no real accounts.
        </p>
      </template>

      <template #right>
        <UButton
          to="https://github.com/farismnrr/ai-code"
          target="_blank"
          icon="i-simple-icons-github"
          aria-label="GitHub"
          color="neutral"
          variant="ghost"
        />
      </template>
    </UFooter>
  </div>
</template>
