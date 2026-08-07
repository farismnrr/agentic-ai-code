<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui'

const items: NavigationMenuItem[] = [
  { label: 'General', icon: 'i-lucide-settings-2', to: '/settings/general' },
  { label: 'Models', icon: 'i-lucide-sparkles', to: '/settings/models' },
  { label: 'MCP', icon: 'i-lucide-blocks', to: '/settings/mcp' },
  { label: 'Account', icon: 'i-lucide-user', to: '/settings/account' }
]
</script>

<template>
  <UDashboardPanel id="settings">
    <template #header>
      <UDashboardNavbar title="Settings">
        <template #leading>
          <UDashboardSidebarToggle />
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
