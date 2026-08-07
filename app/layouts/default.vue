<script setup lang="ts">
import type { DropdownMenuItem, NavigationMenuItem } from '@nuxt/ui'

const { sorted, remove, update, loadAll: loadConversations } = useConversations()
const { workspaces, activeWorkspaceId, create: createWorkspace, loadAll: loadWorkspaces } = useWorkspaces()
const { load: loadSettings } = useSettings()
const { loadAll: loadMcpServers } = useMcpServers()
const { user, logout } = useAuth()
const router = useRouter()
const route = useRoute()

await useAsyncData('app-data', async () => {
  if (user.value) {
    await Promise.all([
      loadSettings(),
      loadWorkspaces().then(loadConversations),
      loadMcpServers()
    ])
  }
  return true
})

// Whenever workspace changes, we should reload conversations
watch(activeWorkspaceId, () => {
  loadConversations()
})

const groups = computed(() => groupConversations(sorted.value))

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

function startRename(id: string, title: string) {
  renaming.value = { id, title }
}

function confirmRename() {
  const pending = renaming.value
  if (!pending) return
  const title = pending.title.trim()
  if (title) update(pending.id, { title })
  renaming.value = null
}

function rowItems(id: string, title: string): DropdownMenuItem[][] {
  return [[
    {
      label: 'Rename',
      icon: 'i-lucide-pencil',
      onSelect: () => startRename(id, title)
    },
    {
      label: 'Delete',
      icon: 'i-lucide-trash-2',
      color: 'error',
      onSelect: () => deleteConversation(id)
    }
  ]]
}

const userItems = computed<DropdownMenuItem[][]>(() => [
  [{ label: 'Settings', icon: 'i-lucide-settings', to: '/settings/general' }],
  [{
    label: 'Sign out',
    icon: 'i-lucide-log-out',
    color: 'error',
    onSelect: () => {
      logout()
      void router.push('/login')
    }
  }]
])

const activeWorkspace = computed(() => workspaces.value.find(w => w.id === activeWorkspaceId.value))

const workspaceCreating = ref(false)
const workspaceName = ref('')
async function confirmCreateWorkspace() {
  if (workspaceName.value.trim()) {
    const w = await createWorkspace(workspaceName.value.trim())
    activeWorkspaceId.value = w.id
  }
  workspaceCreating.value = false
  workspaceName.value = ''
}

const workspaceItems = computed<DropdownMenuItem[][]>(() => {
  const list = workspaces.value.map(w => ({
    label: w.name,
    icon: activeWorkspaceId.value === w.id ? 'i-lucide-check' : 'i-lucide-folder',
    onSelect: () => { activeWorkspaceId.value = w.id }
  }))
  return [
    list,
    [{ label: 'New workspace', icon: 'i-lucide-plus', onSelect: () => { workspaceCreating.value = true } }]
  ]
})

const searchOpen = ref(false)
defineShortcuts({
  meta_k: () => { searchOpen.value = true },
  meta_shift_o: newChat
})

const searchGroups = computed(() => [
  {
    id: 'workspaces',
    label: 'Workspaces',
    items: workspaces.value.map(w => ({
      label: w.name,
      icon: 'i-lucide-folder',
      onSelect: () => { activeWorkspaceId.value = w.id }
    }))
  },
  {
    id: 'conversations',
    label: 'Conversations',
    items: sorted.value.map(conversation => ({
      label: conversation.title,
      suffix: new Date(conversation.updatedAt).toLocaleDateString(),
      icon: 'i-lucide-message-square',
      to: `/chat/${conversation.id}`
    }))
  }
])
</script>

<template>
  <UDashboardGroup>
    <UDashboardSidebar
      collapsible
      resizable
      :min-size="14"
      :default-size="17"
      :max-size="26"
    >
      <template #header="{ collapsed }">
        <UDropdownMenu
          :items="workspaceItems"
          class="w-full"
        >
          <UButton
            :label="collapsed ? undefined : (activeWorkspace?.name ?? 'Workspace')"
            :square="collapsed"
            icon="i-lucide-layout-grid"
            color="neutral"
            variant="ghost"
            :block="!collapsed"
            :trailing-icon="collapsed ? undefined : 'i-lucide-chevron-down'"
            :ui="{ trailingIcon: 'ms-auto' }"
          />
        </UDropdownMenu>
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
            v-for="group in groups"
            :key="group.label"
            class="mb-3"
          >
            <p class="px-2.5 py-1 text-xs font-medium text-dimmed">
              {{ group.label }}
            </p>

            <UNavigationMenu
              :items="itemsFor(group.conversations)"
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

          <div
            v-if="!groups.length"
            class="px-2.5 py-4 text-center"
          >
            <p class="mb-3 text-sm text-muted">
              No conversations yet.
            </p>
            <UButton
              label="Start one"
              icon="i-lucide-message-square-plus"
              color="neutral"
              variant="outline"
              size="xs"
              @click="newChat"
            />
          </div>
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
      </template>

      <template #footer>
        <div class="flex w-full justify-end gap-2">
          <UButton
            label="Cancel"
            color="neutral"
            variant="ghost"
            @click="renaming = null"
          />
          <UButton
            label="Rename"
            @click="confirmRename"
          />
        </div>
      </template>
    </UModal>

    <UModal
      :open="workspaceCreating"
      title="New workspace"
      @update:open="workspaceCreating = false"
    >
      <template #body>
        <UInput
          v-model="workspaceName"
          autofocus
          placeholder="Workspace name..."
          class="w-full"
          @keydown.enter="confirmCreateWorkspace"
        />
      </template>

      <template #footer>
        <div class="flex w-full justify-end gap-2">
          <UButton
            label="Cancel"
            color="neutral"
            variant="ghost"
            @click="workspaceCreating = false"
          />
          <UButton
            label="Create"
            @click="confirmCreateWorkspace"
          />
        </div>
      </template>
    </UModal>

    <div class="flex flex-1 flex-col min-w-0 h-full">
      <div
        v-if="user && !user.emailVerifiedAt"
        class="bg-primary-500/10 border-b border-primary-500/20 px-4 py-3 flex items-center justify-between shrink-0 z-50"
      >
        <div class="flex items-center gap-3">
          <UIcon
            name="i-lucide-mail-warning"
            class="w-5 h-5 text-primary"
          />
          <p class="text-sm font-medium text-primary">
            Please verify your email address to secure your account.
          </p>
        </div>
      </div>
      <slot />
    </div>
  </UDashboardGroup>
</template>
