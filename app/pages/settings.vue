<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui'

// General, Models and Account land in phase 5; listing them now would link
// to routes that 404.
const items: NavigationMenuItem[] = [
  { label: 'MCP', icon: 'i-lucide-blocks', to: '/settings/mcp' }
]
</script>

<template>
  <UDashboardPanel id="settings">
    <template #header>
      <UDashboardNavbar title="Settings">
        <template #leading>
          <UDashboardSidebarCollapse />
        </template>
      </UDashboardNavbar>

      <UDashboardToolbar>
        <UNavigationMenu
          :items="items"
          highlight
          class="-mb-px flex-1"
        />
      </UDashboardToolbar>
    </template>

    <template #body>
      <UContainer class="py-2">
        <NuxtPage />
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
