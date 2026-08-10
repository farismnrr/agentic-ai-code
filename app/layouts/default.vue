<script setup lang="ts">
import type { DropdownMenuItem, NavigationMenuItem } from '@nuxt/ui'

const { sorted, remove, update, loadOne } = useConversations()
const { workspaces, activeWorkspaceId, create: createWorkspace, remove: removeWorkspace, update: updateWorkspace, setActive } = useWorkspaces()
const { load: loadSidebar, pending: sidebarPending, error: sidebarError } = useSidebarData()
const settings = useSettings()
const loadSettings = settings.load
const { loadAll: loadMcpServers } = useMcpServers()
const { user, logout } = useAuth()
const router = useRouter()
const route = useRoute()

const { refresh: refreshAppData } = useAsyncData('app-data', async () => {
  if (user.value) {
    // Opening a conversation directly (a deep link, a bookmark, a refresh)
    // never went through the workspace picker, so activeWorkspaceId can be
    // unset or stale even though the conversation itself belongs to a real
    // workspace. This has to be resolved and set *before* the sidebar
    // renders — the sidebar's own template renders once this resolves, and
    // by then it's too late for a page component further down the tree to
    // correct it; SSR doesn't re-render a parent because a child mutated
    // shared state after the parent already rendered.
    const chatRouteMatch = route.path.match(/^\/chat\/([^/]+)$/)
    // loadOne() must be *invoked* here, not awaited before the batch below —
    // it calls useRequestFetch() internally, which needs Nuxt's SSR context.
    // Awaiting it first crosses an await boundary before loadSidebar()/
    // loadMcpServers() are invoked, breaking their own internal composable
    // calls (NUXT_E1001, silently swallowed by Promise.allSettled — see
    // .agents/memories/015-composable-after-await-breaks-ssr-context.md).
    // Chaining .then() here keeps the invocation synchronous while still
    // resolving setActive() before this whole block returns.
    const loadOnePromise = chatRouteMatch
      ? loadOne(chatRouteMatch[1]!).then((conv) => {
          if (conv && conv.workspaceId !== activeWorkspaceId.value) {
            setActive(conv.workspaceId)
          }
        })
      : Promise.resolve()

    // Sidebar data (workspaces + conversation metadata) is one server-side
    // join (GET /api/sidebar via useSidebarData()), not two client-side
    // composable calls orchestrated here — see
    // .agents/memories/018-sidebar-single-fetch.md for why that used to
    // break. loadSidebar() awaits this same settings promise *internally*,
    // for the same SSR-context reason `loadOne` above is invoked (not
    // awaited) synchronously.
    const settingsPromise = loadSettings()
    await Promise.allSettled([
      loadOnePromise,
      settingsPromise,
      loadSidebar(settingsPromise),
      loadMcpServers()
    ])
  }
  return true
}, { lazy: true })

// Whenever workspace changes, we don't necessarily need to reload the
// sidebar since we already load ALL workspaces' conversations up front.
// But keeping it as a full refresh mechanism doesn't hurt.
watch(activeWorkspaceId, (newId, oldId) => {
  if (oldId !== undefined && newId !== oldId) {
    loadSidebar()
  }
})

const workspaceGroups = computed(() => {
  return workspaces.value.map((workspace) => {
    return {
      workspace,
      conversations: sorted.value.filter(c => c.workspaceId === workspace.id)
    }
  })
})

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

const workspaceCreating = ref(false)
const workspaceCreatingPending = ref(false)
const toast = useToast()

async function handleSelectCreateWorkspace(result: { name: string, path: string }) {
  workspaceCreatingPending.value = true
  try {
    const w = await createWorkspace(result.name, result.path)
    setActive(w.id)
    workspaceCreating.value = false
  } catch (err) {
    toast.add({
      title: 'Failed to create workspace',
      description: (err as Error).message,
      color: 'error'
    })
  } finally {
    workspaceCreatingPending.value = false
  }
}

const workspaceConfirming = ref<typeof workspaces.value[0] | null>(null)
const workspaceConfirmingPending = ref(false)

async function handleSelectConfirmWorkspace(result: { name: string, path: string }) {
  if (!workspaceConfirming.value) return
  workspaceConfirmingPending.value = true
  try {
    await updateWorkspace(workspaceConfirming.value.id, { name: result.name, path: result.path })
    workspaceConfirming.value = null
  } catch (err) {
    toast.add({
      title: 'Failed to confirm workspace',
      description: (err as Error).message,
      color: 'error'
    })
  } finally {
    workspaceConfirmingPending.value = false
  }
}

const workspaceRenaming = ref<{ id: string, name: string } | null>(null)
function startRenameWorkspace(id: string, name: string) {
  workspaceRenaming.value = { id, name }
}
function confirmRenameWorkspace() {
  const pending = workspaceRenaming.value
  if (!pending) return
  const name = pending.name.trim()
  if (name) updateWorkspace(pending.id, { name })
  workspaceRenaming.value = null
}

const workspaceDetailsPath = ref<string | null>(null)

function workspaceActionItems(w: typeof workspaces.value[0]): DropdownMenuItem[][] {
  return [[
    {
      label: 'New chat',
      icon: 'i-lucide-square-pen',
      onSelect: () => {
        setActive(w.id)
        void router.push('/chat')
      }
    },
    ...(!w.pathConfirmed
      ? [{
          label: 'Confirm Folder',
          icon: 'i-lucide-alert-circle',
          color: 'warning' as const,
          onSelect: () => { workspaceConfirming.value = w }
        }]
      : []),
    {
      label: 'View details',
      icon: 'i-lucide-info',
      onSelect: () => { workspaceDetailsPath.value = w.path }
    },
    {
      label: 'Rename',
      icon: 'i-lucide-pencil',
      onSelect: () => startRenameWorkspace(w.id, w.name)
    },
    {
      label: 'Delete',
      icon: 'i-lucide-trash-2',
      color: 'error' as const,
      onSelect: async () => {
        await removeWorkspace(w.id)
        if (activeWorkspaceId.value === w.id) {
          setActive(workspaces.value[0]?.id || null)
        }
      }
    }
  ]]
}

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
      onSelect: () => { setActive(w.id) }
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
            v-if="sidebarPending"
            class="px-2.5 py-2 space-y-4"
          >
            <div
              v-for="i in 2"
              :key="i"
              class="space-y-2"
            >
              <USkeleton class="h-4 w-28 rounded" />
              <USkeleton class="h-6 w-full rounded" />
              <USkeleton class="h-6 w-full rounded" />
            </div>
          </div>

          <div
            v-else-if="sidebarError"
            class="px-2"
          >
            <DataLoadError
              title="Couldn't load workspaces"
              description="Failed to load your workspaces and conversations."
              @retry="refreshAppData()"
            />
          </div>

          <template v-else>
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

              <!-- One UNavigationMenu per workspace -->
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
              v-if="workspaces.length === 0"
              class="px-2.5 py-4 text-center"
            >
              <p class="mb-3 text-sm text-muted">
                No workspaces yet.
              </p>
              <UButton
                label="Create one"
                icon="i-lucide-folder-plus"
                color="neutral"
                variant="outline"
                size="xs"
                @click="workspaceCreating = true"
              />
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

    <!-- Renaming happens in a modal, rather than inline in the sidebar, to keep the sidebar HTML simple -->
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

    <WorkspaceFolderPicker
      v-model="workspaceCreating"
      :pending="workspaceCreatingPending"
      @select="handleSelectCreateWorkspace"
    />

    <WorkspaceFolderPicker
      :model-value="!!workspaceConfirming"
      :initial-name="workspaceConfirming?.name"
      :initial-path="workspaceConfirming?.path"
      :is-update="true"
      :pending="workspaceConfirmingPending"
      @update:model-value="(val) => { if (!val) workspaceConfirming = null }"
      @select="handleSelectConfirmWorkspace"
    />

    <UModal
      :open="workspaceRenaming !== null"
      title="Rename workspace"
      @update:open="workspaceRenaming = null"
    >
      <template #body>
        <UInput
          v-if="workspaceRenaming"
          v-model="workspaceRenaming.name"
          autofocus
          class="w-full"
          @keydown.enter="confirmRenameWorkspace"
        />
      </template>

      <template #footer>
        <div class="flex w-full justify-end gap-2">
          <UButton
            label="Cancel"
            color="neutral"
            variant="ghost"
            @click="workspaceRenaming = null"
          />
          <UButton
            label="Rename"
            @click="confirmRenameWorkspace"
          />
        </div>
      </template>
    </UModal>

    <UModal
      :open="workspaceDetailsPath !== null"
      title="Workspace Details"
      @update:open="workspaceDetailsPath = null"
    >
      <template #body>
        <p class="text-sm font-mono break-all text-default bg-elevated p-2 rounded border border-muted">
          {{ workspaceDetailsPath }}
        </p>
      </template>
      <template #footer>
        <div class="flex w-full justify-end">
          <UButton
            label="Close"
            color="neutral"
            @click="workspaceDetailsPath = null"
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
