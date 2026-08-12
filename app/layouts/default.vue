<script setup lang="ts">
const { loadOne } = useConversations()
const { activeWorkspaceId, setActive } = useWorkspaces()
const { load: loadSidebar, pending: sidebarPending, error: sidebarError } = useSidebarData()
const settings = useSettings()
const { loadAll: loadMcpServers } = useMcpServers()
const { user } = useAuth()
const route = useRoute()

const { refresh: refreshAppData } = useAsyncData('app-data', async () => {
  if (user.value) {
    const chatRouteMatch = route.path.match(/^\/chat\/([^/]+)$/)
    const loadOnePromise = chatRouteMatch
      ? loadOne(chatRouteMatch[1]!).then((conversation) => {
          if (conversation && conversation.workspaceId !== activeWorkspaceId.value) setActive(conversation.workspaceId)
        })
      : Promise.resolve()
    const settingsPromise = settings.load()
    await Promise.allSettled([loadOnePromise, settingsPromise, loadSidebar(settingsPromise), loadMcpServers()])
  }
  return true
}, { lazy: true })

watch(activeWorkspaceId, (newId, oldId) => {
  if (oldId !== undefined && newId !== oldId) loadSidebar()
})
</script>

<template>
  <UDashboardGroup>
    <AppSidebar
      :refresh-app-data="refreshAppData"
      :sidebar-pending="sidebarPending"
      :sidebar-error="sidebarError"
    />
    <div class="flex flex-1 flex-col min-w-0 h-full">
      <div
        v-if="user && !user.emailVerifiedAt"
        class="bg-primary-500/10 border-b border-primary-500/20 px-4 py-3 flex items-center justify-between shrink-0 z-50"
      >
        <div class="flex items-center gap-3">
          <UIcon
            name="i-lucide-mail-warning"
            class="w-5 h-5 text-primary"
          /><p class="text-sm font-medium text-primary">
            Please verify your email address to secure your account.
          </p>
        </div>
      </div>
      <slot />
    </div>
  </UDashboardGroup>
</template>
