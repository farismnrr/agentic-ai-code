<script setup lang="ts">
const route = useRoute()
const { get } = useConversations()

const conversation = computed(() => get(String(route.params.id)))

useSeoMeta({ title: () => conversation.value?.title ?? 'Chat' })
</script>

<template>
  <UDashboardPanel :id="`chat-${route.params.id}`">
    <template #header>
      <UDashboardNavbar :title="conversation?.title ?? 'Chat'">
        <template #leading>
          <UDashboardSidebarCollapse />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <div
        v-if="!conversation"
        class="flex flex-1 items-center justify-center"
      >
        <UAlert
          icon="i-lucide-message-square-off"
          title="Conversation not found"
          description="It may have been deleted, or the link is stale."
          color="neutral"
          variant="subtle"
          class="max-w-md"
        />
      </div>

      <div
        v-else
        class="flex flex-1 items-center justify-center"
      >
        <p class="text-muted">
          {{ conversation.messages.length }} messages — rendering lands in phase 3.
        </p>
      </div>
    </template>
  </UDashboardPanel>
</template>
