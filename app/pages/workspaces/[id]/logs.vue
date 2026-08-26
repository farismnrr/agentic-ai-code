<script setup lang="ts">
import type { ActivityResponse } from '#shared/types/activity'

const route = useRoute()
const workspaceId = computed(() => String(route.params.id))
const { data, status, error, refresh } = await useFetch<ActivityResponse>(() => `/api/workspaces/${workspaceId.value}/activity`, {
  query: { limit: 30, ...route.query },
  default: () => ({ items: [], nextCursor: null, hasMore: false })
})

useSeoMeta({ title: 'Workspace logs' })
</script>

<template>
  <UDashboardPanel id="workspace-logs">
    <template #header>
      <UDashboardNavbar title="Workspace logs">
        <template #left>
          <UDashboardSidebarCollapse />
        </template>
      </UDashboardNavbar>
    </template>
    <template #body>
      <WorkspaceActivityView
        :workspace-id="workspaceId"
        :initial-data="data"
        :initial-status="status"
        :initial-error="error"
        @refresh="refresh"
      />
    </template>
  </UDashboardPanel>
</template>
