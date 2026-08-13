<script setup lang="ts">
const creating = defineModel<boolean>('creating', { required: true })
const confirmingWorkspace = defineModel<{ id: string, name: string, path: string } | null>('confirmingWorkspace', { required: true })
const renamingWorkspace = defineModel<{ id: string, name: string } | null>('renamingWorkspace', { required: true })
const detailsPath = defineModel<string | null>('detailsPath', { required: true })

const { create: createWorkspace, update: updateWorkspace, setActive } = useWorkspaces()
const toast = useToast()

const creatingPending = ref(false)
async function handleCreate(result: { name: string, path: string }) {
  creatingPending.value = true
  try {
    const workspace = await createWorkspace(result.name, result.path)
    setActive(workspace.id)
    creating.value = false
  } catch (err) {
    toast.add({ title: 'Failed to create workspace', description: (err as Error).message, color: 'error' })
  } finally {
    creatingPending.value = false
  }
}

const confirmingPending = ref(false)
async function handleConfirm(result: { name: string, path: string }) {
  if (!confirmingWorkspace.value) return
  confirmingPending.value = true
  try {
    await updateWorkspace(confirmingWorkspace.value.id, { name: result.name, path: result.path })
    confirmingWorkspace.value = null
  } catch (err) {
    toast.add({ title: 'Failed to confirm workspace', description: (err as Error).message, color: 'error' })
  } finally {
    confirmingPending.value = false
  }
}

function confirmRename() {
  const pending = renamingWorkspace.value
  if (!pending) return
  const name = pending.name.trim()
  if (name) updateWorkspace(pending.id, { name })
  renamingWorkspace.value = null
}
</script>

<template>
  <WorkspaceFolderPicker
    v-model="creating"
    :pending="creatingPending"
    @select="handleCreate"
  />
  <WorkspaceFolderPicker
    :model-value="!!confirmingWorkspace"
    :initial-name="confirmingWorkspace?.name"
    :initial-path="confirmingWorkspace?.path"
    :is-update="true"
    :pending="confirmingPending"
    @update:model-value="(value) => { if (!value) confirmingWorkspace = null }"
    @select="handleConfirm"
  />
  <UModal
    :open="renamingWorkspace !== null"
    title="Rename workspace"
    @update:open="renamingWorkspace = null"
  >
    <template #body>
      <UInput
        v-if="renamingWorkspace"
        v-model="renamingWorkspace.name"
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
          @click="renamingWorkspace = null"
        /><UButton
          label="Rename"
          @click="confirmRename"
        />
      </div>
    </template>
  </UModal>
  <UModal
    :open="detailsPath !== null"
    title="Workspace Details"
    @update:open="detailsPath = null"
  >
    <template #body>
      <p class="text-sm font-mono break-all text-default bg-elevated p-2 rounded border border-muted">
        {{ detailsPath }}
      </p>
    </template><template #footer>
      <div class="flex w-full justify-end">
        <UButton
          label="Close"
          color="neutral"
          @click="detailsPath = null"
        />
      </div>
    </template>
  </UModal>
</template>
