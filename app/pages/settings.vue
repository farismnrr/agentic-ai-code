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

      <!-- The toolbar ships `overflow-x-auto` so it can scroll on narrow
           screens. CSS then computes the visible `overflow-y` to `auto` as
           well — when one axis is not visible, the other can't stay visible.
           The `-mb-px` below pushes the tab indicator 1px past the container
           to sit on its border, and that 1px was enough to grow a vertical
           scrollbar. Clipping the y axis keeps the horizontal scrolling that
           was intended and drops the scrollbar that wasn't. -->
      <UDashboardToolbar :ui="{ root: 'overflow-y-hidden' }">
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
