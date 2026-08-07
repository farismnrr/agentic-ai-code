<script setup lang="ts">
import type { DropdownMenuItem, NavigationMenuItem } from '@nuxt/ui'

const { sorted, remove } = useConversations()
const router = useRouter()
const route = useRoute()

const groups = computed(() => groupConversations(sorted.value))

/**
 * One `UNavigationMenu` per bucket. Passing every bucket to a single menu
 * would flatten the headings, and the grouping is the whole point of the list.
 */
function itemsFor(conversations: typeof sorted.value): NavigationMenuItem[] {
  return conversations.map(conversation => ({
    label: conversation.title,
    to: `/c/${conversation.id}`,
    active: route.path === `/c/${conversation.id}`,
    // Carried through so the row's delete button knows what to remove.
    value: conversation.id
  }))
}

function newChat() {
  void router.push('/')
}

function deleteConversation(id: string) {
  const wasOpen = route.path === `/c/${id}`
  remove(id)
  if (wasOpen) void router.push('/')
}

const userItems: DropdownMenuItem[][] = [
  [{ label: 'Settings', icon: 'i-lucide-settings', to: '/settings/general' }],
  [{ label: 'Sign out', icon: 'i-lucide-log-out', color: 'error' }]
]

// ⌘K opens search, ⌘⇧O starts a new chat — the two shortcuts people expect.
const searchOpen = ref(false)
defineShortcuts({
  meta_k: () => { searchOpen.value = true },
  meta_shift_o: newChat
})

const searchGroups = computed(() => [{
  id: 'conversations',
  label: 'Conversations',
  items: sorted.value.map(conversation => ({
    label: conversation.title,
    suffix: new Date(conversation.updatedAt).toLocaleDateString(),
    icon: 'i-lucide-message-square',
    to: `/c/${conversation.id}`
  }))
}])
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
        <UButton
          :label="collapsed ? undefined : 'New chat'"
          :square="collapsed"
          icon="i-lucide-square-pen"
          color="neutral"
          variant="outline"
          :block="!collapsed"
          @click="newChat"
        />
      </template>

      <template #default="{ collapsed }">
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
              :ui="{ link: 'group' }"
            >
              <template #item-trailing="{ item }">
                <UButton
                  icon="i-lucide-trash-2"
                  color="neutral"
                  variant="ghost"
                  size="xs"
                  class="opacity-0 group-hover:opacity-100"
                  :aria-label="`Delete ${item.label}`"
                  @click.stop.prevent="deleteConversation(String(item.value))"
                />
              </template>
            </UNavigationMenu>
          </div>

          <p
            v-if="!groups.length"
            class="px-2.5 py-4 text-sm text-muted"
          >
            No conversations yet.
          </p>
        </template>
      </template>

      <template #footer="{ collapsed }">
        <UDropdownMenu
          :items="userItems"
          class="w-full"
        >
          <UButton
            :label="collapsed ? undefined : 'Faris'"
            :square="collapsed"
            :avatar="{ alt: 'Faris' }"
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

    <slot />
  </UDashboardGroup>
</template>
