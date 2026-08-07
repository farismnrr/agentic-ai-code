<script setup lang="ts">
// Placeholder shell for the empty state. Phase 3 replaces this with the
// greeting, suggested prompts and the centred prompt box.
useSeoMeta({ title: 'New chat' })
</script>

<template>
  <UDashboardPanel id="home">
    <template #header>
      <UDashboardNavbar title="New chat">
        <template #leading>
          <UDashboardSidebarCollapse />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <div class="flex flex-1 items-center justify-center">
        <p class="text-muted">
          Chat view lands in phase 3.
        </p>
      </div>
    </template>
  </UDashboardPanel>
</template>
