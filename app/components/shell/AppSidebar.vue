<script setup lang="ts">
import type { DropdownMenuItem, NavigationMenuItem } from '@nuxt/ui'

const props = defineProps<{ refreshAppData: () => unknown, sidebarPending: boolean, sidebarError: Error | null }>()
const { sorted, remove, update } = useConversations()
const { workspaces, activeWorkspaceId, remove: removeWorkspace, setActive } = useWorkspaces()
const { user, logout } = useAuth()
const router = useRouter()
const route = useRoute()

const workspaceGroups = computed(() => workspaces.value.map(workspace => ({
  workspace,
  conversations: sorted.value.filter(conversation => conversation.workspaceId === workspace.id)
})))

function itemsFor(conversations: typeof sorted.value): NavigationMenuItem[] {
  return conversations.map(conversation => ({
    label: conversation.title,
    to: `/chat/${conversation.id}`,
    active: route.path === `/chat/${conversation.id}`,
    value: conversation.id
  }))
}

function newChat() {
  void router.push('/chat')
}

function deleteConversation(id: string) {
  const wasOpen = route.path === `/chat/${id}`
  remove(id)
  if (wasOpen) void router.push('/chat')
}

const renaming = ref<{ id: string, title: string } | null>(null)
function confirmRename() {
  const pending = renaming.value
  if (!pending) return
  const title = pending.title.trim()
  if (title) update(pending.id, { title })
  renaming.value = null
}
function rowItems(id: string, title: string): DropdownMenuItem[][] {
  return [[
    { label: 'Rename', icon: 'i-lucide-pencil', onSelect: () => { renaming.value = { id, title } } },
    { label: 'Delete', icon: 'i-lucide-trash-2', color: 'error', onSelect: () => deleteConversation(id) }
  ]]
}

const workspaceCreating = ref(false)
const workspaceConfirming = ref<typeof workspaces.value[0] | null>(null)
const workspaceRenaming = ref<{ id: string, name: string } | null>(null)
const workspaceDetailsPath = ref<string | null>(null)
function workspaceActionItems(workspace: typeof workspaces.value[0]): DropdownMenuItem[][] {
  return [[
    { label: 'New chat', icon: 'i-lucide-square-pen', onSelect: () => {
      setActive(workspace.id)
      void router.push('/chat')
    } },
    ...(!workspace.pathConfirmed ? [{ label: 'Confirm Folder', icon: 'i-lucide-alert-circle', color: 'warning' as const, onSelect: () => { workspaceConfirming.value = workspace } }] : []),
    { label: 'View details', icon: 'i-lucide-info', onSelect: () => { workspaceDetailsPath.value = workspace.path } },
    { label: 'Rename', icon: 'i-lucide-pencil', onSelect: () => { workspaceRenaming.value = { id: workspace.id, name: workspace.name } } },
    { label: 'Delete', icon: 'i-lucide-trash-2', color: 'error' as const, onSelect: async () => {
      await removeWorkspace(workspace.id)
      if (activeWorkspaceId.value === workspace.id) setActive(workspaces.value[0]?.id || null)
    } }
  ]]
}

const searchOpen = ref(false)
defineShortcuts({
  meta_k: () => { searchOpen.value = true },
  meta_shift_o: newChat
})
const searchGroups = computed(() => [
  { id: 'workspaces', label: 'Workspaces', items: workspaces.value.map(workspace => ({ label: workspace.name, icon: 'i-lucide-folder', onSelect: () => { setActive(workspace.id) } })) },
  { id: 'conversations', label: 'Conversations', items: sorted.value.map(conversation => ({ label: conversation.title, suffix: new Date(conversation.updatedAt).toLocaleDateString(), icon: 'i-lucide-message-square', to: `/chat/${conversation.id}` })) }
])
const userItems = computed<DropdownMenuItem[][]>(() => [
  [{ label: 'Settings', icon: 'i-lucide-settings', to: '/settings/general' }],
  [{ label: 'Sign out', icon: 'i-lucide-log-out', color: 'error', onSelect: () => {
    logout()
    void router.push('/login')
  } }]
])
</script>

<template>
  <UDashboardSidebar
    collapsible
    resizable
    :min-size="14"
    :default-size="17"
    :max-size="26"
  >
    <template #header="{ collapsed }">
      <div class="flex items-center justify-between px-2 w-full">
        <span
          v-if="!collapsed"
          class="font-semibold text-sm truncate"
        >Workspaces</span>
        <UButton
          icon="i-lucide-plus"
          color="neutral"
          variant="ghost"
          size="xs"
          :class="collapsed ? 'mx-auto' : ''"
          :title="collapsed ? 'New workspace' : undefined"
          @click="workspaceCreating = true"
        />
      </div>
    </template>
    <template #default="{ collapsed }">
      <div class="px-2 pb-2">
        <UButton
          :label="collapsed ? undefined : 'New chat'"
          :square="collapsed"
          icon="i-lucide-square-pen"
          color="neutral"
          variant="outline"
          :block="!collapsed"
          @click="newChat"
        />
      </div>
      <UDashboardSearchButton
        :collapsed="collapsed"
        class="mb-2"
      />
      <template v-if="!collapsed">
        <div
          v-if="props.sidebarPending"
          class="px-2.5 py-2 space-y-4"
        >
          <div
            v-for="i in 2"
            :key="i"
            class="space-y-2"
          >
            <USkeleton class="h-4 w-28 rounded" /><USkeleton class="h-6 w-full rounded" /><USkeleton class="h-6 w-full rounded" />
          </div>
        </div>
        <div
          v-else-if="props.sidebarError"
          class="px-2"
        >
          <DataLoadError
            title="Couldn't load workspaces"
            description="Failed to load your workspaces and conversations."
            @retry="props.refreshAppData"
          />
        </div>
        <template v-else>
          <div
            v-if="workspaceGroups.length === 0"
            class="px-2.5 py-4 text-center"
          >
            <p class="mb-3 text-sm text-muted">
              No workspaces yet.
            </p><UButton
              label="Create one"
              icon="i-lucide-folder-plus"
              color="neutral"
              variant="outline"
              size="xs"
              @click="workspaceCreating = true"
            />
          </div>
          <div
            v-for="group in workspaceGroups"
            :key="group.workspace.id"
            class="mb-3"
          >
            <div class="px-2.5 py-1 flex items-center justify-between group">
              <p
                class="text-xs font-medium cursor-pointer truncate mr-2"
                :class="activeWorkspaceId === group.workspace.id ? 'text-primary' : 'text-dimmed hover:text-primary'"
                :title="group.workspace.name"
                @click="setActive(group.workspace.id)"
              >
                {{ group.workspace.name }}
              </p>
              <UDropdownMenu :items="workspaceActionItems(group.workspace)">
                <UButton
                  icon="i-lucide-ellipsis"
                  color="neutral"
                  variant="ghost"
                  size="xs"
                  class="opacity-0 group-hover:opacity-100"
                  @click.stop.prevent
                />
              </UDropdownMenu>
            </div>
            <UNavigationMenu
              :items="[
                { label: 'Logs', icon: 'i-lucide-activity', to: `/workspaces/${group.workspace.id}/logs`, active: route.path === `/workspaces/${group.workspace.id}/logs` },
                ...itemsFor(group.conversations)
              ]"
              orientation="vertical"
              :ui="{ link: 'group', root: 'py-0.5' }"
            >
              <template #item-trailing="{ item }">
                <UDropdownMenu :items="rowItems(String(item.value), String(item.label))">
                  <UButton
                    icon="i-lucide-ellipsis"
                    color="neutral"
                    variant="ghost"
                    size="xs"
                    :aria-label="`Options for ${item.label}`"
                    @click.stop.prevent
                  />
                </UDropdownMenu>
              </template>
            </UNavigationMenu>
          </div>
        </template>
      </template>
    </template>
    <template #footer="{ collapsed }">
      <UDropdownMenu
        :items="userItems"
        class="w-full"
      >
        <UButton
          :label="collapsed ? undefined : (user?.name ?? 'Account')"
          :square="collapsed"
          :avatar="{ alt: user?.name ?? 'Account' }"
          color="neutral"
          variant="ghost"
          :block="!collapsed"
          :trailing-icon="collapsed ? undefined : 'i-lucide-chevrons-up-down'"
          :ui="{ trailingIcon: 'ms-auto' }"
        />
      </UDropdownMenu>
    </template>
  </UDashboardSidebar>
  <UDashboardSearch
    v-model:open="searchOpen"
    :groups="searchGroups"
  />
  <UModal
    :open="renaming !== null"
    title="Rename conversation"
    @update:open="renaming = null"
  >
    <template #body>
      <UInput
        v-if="renaming"
        v-model="renaming.title"
        autofocus
        class="w-full"
        @keydown.enter="confirmRename"
      />
    </template><template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton
          label="Cancel"
          color="neutral"
          variant="ghost"
          @click="renaming = null"
        /><UButton
          label="Rename"
          @click="confirmRename"
        />
      </div>
    </template>
  </UModal>
  <ShellAppSidebarWorkspaceDialogs
    v-model:creating="workspaceCreating"
    v-model:confirming-workspace="workspaceConfirming"
    v-model:renaming-workspace="workspaceRenaming"
    v-model:details-path="workspaceDetailsPath"
  />
</template>
