<script setup lang="ts">
const { workspaces, create: createWorkspace, update: updateWorkspace, setActive } = useWorkspaces()

const workspaceCreating = ref(false)
const pending = ref(false)
const toast = useToast()

const workspaceConfirming = ref<typeof workspaces.value[0] | null>(null)

async function handleSelectFolder(result: { name: string, path: string }) {
  pending.value = true
  try {
    if (workspaceConfirming.value) {
      await updateWorkspace(workspaceConfirming.value.id, { name: result.name, path: result.path })
      workspaceConfirming.value = null
    } else {
      const w = await createWorkspace(result.name, result.path)
      setActive(w.id)
      workspaceCreating.value = false
    }
  } catch (err) {
    toast.add({
      title: workspaceConfirming.value ? 'Failed to update workspace' : 'Failed to create workspace',
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
        @click="setActive(workspace.id)"
      >
        <template
          v-if="!workspace.pathConfirmed"
          #description
        >
          <div class="mt-2">
            <UButton
              size="xs"
              color="warning"
              variant="soft"
              icon="i-lucide-alert-circle"
              @click.stop="workspaceConfirming = workspace"
            >
              Confirm Folder
            </UButton>
          </div>
        </template>
      </UPageCard>

      <UPageCard
        title="Create workspace"
        icon="i-lucide-plus"
        class="cursor-pointer border-dashed"
        @click="workspaceCreating = true"
      />
    </UPageGrid>

    <WorkspaceFolderPicker
      v-model="workspaceCreating"
      :pending="pending"
      @select="handleSelectFolder"
    />

    <WorkspaceFolderPicker
      :model-value="!!workspaceConfirming"
      :initial-name="workspaceConfirming?.name"
      :initial-path="workspaceConfirming?.path"
      :is-update="true"
      :pending="pending"
      @update:model-value="(val) => { if (!val) workspaceConfirming = null }"
      @select="handleSelectFolder"
    />
  </UContainer>
</template>
