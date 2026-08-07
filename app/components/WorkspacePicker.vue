<script setup lang="ts">
const { workspaces, activeWorkspaceId, create: createWorkspace } = useWorkspaces()

const workspaceCreating = ref(false)
const workspaceName = ref('')
const pending = ref(false)
const toast = useToast()

async function confirmCreateWorkspace() {
  const name = workspaceName.value.trim()
  if (!name) return

  pending.value = true
  try {
    const w = await createWorkspace(name)
    activeWorkspaceId.value = w.id
    workspaceCreating.value = false
    workspaceName.value = ''
  } catch (err) {
    toast.add({
      title: 'Failed to create workspace',
      description: (err as Error).message,
      color: 'error'
    })
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <UContainer class="flex w-full flex-1 flex-col justify-center gap-8 py-10">
    <div class="text-center">
      <h1 class="text-2xl font-semibold text-highlighted sm:text-3xl">
        Pick a workspace
      </h1>
      <p class="mt-2 text-muted">
        Select a workspace to start chatting, or create a new one.
      </p>
    </div>

    <UPageGrid>
      <UPageCard
        v-for="workspace in workspaces"
        :key="workspace.id"
        :title="workspace.name"
        icon="i-lucide-folder"
        class="cursor-pointer hover:border-primary-500/50 transition-colors"
        @click="activeWorkspaceId = workspace.id"
      />

      <UPageCard
        title="Create workspace"
        icon="i-lucide-plus"
        class="cursor-pointer border-dashed"
        @click="workspaceCreating = true"
      />
    </UPageGrid>

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
          :loading="pending"
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
            :loading="pending"
            @click="confirmCreateWorkspace"
          />
        </div>
      </template>
    </UModal>
  </UContainer>
</template>
