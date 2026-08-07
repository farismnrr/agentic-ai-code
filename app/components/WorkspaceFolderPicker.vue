<script setup lang="ts">
import { ref, computed, watch } from 'vue'

const props = defineProps<{
  modelValue: boolean
  initialPath?: string
  initialName?: string
  isUpdate?: boolean
  pending?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'select': [{ name: string, path: string }]
}>()

const isOpen = computed({
  get: () => props.modelValue,
  set: val => emit('update:modelValue', val)
})

const currentPath = ref(props.initialPath || '')
const workspaceName = ref(props.initialName || '')
const rootName = ref('~')
const directories = ref<{ name: string, path: string }[]>([])
const loading = ref(false)
const toast = useToast()

async function loadPath(path: string) {
  loading.value = true
  try {
    const { data, error } = await useFetch('/api/fs/browse', { query: { path } })
    if (error.value) throw new Error(error.value.message || 'Failed to load directory')
    if (data.value) {
      directories.value = data.value.entries || []
      currentPath.value = data.value.path || ''
      // Get the basename of the root if possible
      const rootParts = (data.value.root || '').split(/[/\\]/).filter(Boolean)
      rootName.value = rootParts.length ? (rootParts[rootParts.length - 1] || '~') : '~'
    }
  } catch (err) {
    toast.add({ title: 'Error', description: (err as Error).message, color: 'error' })
  } finally {
    loading.value = false
  }
}

watch(isOpen, (val) => {
  if (val) {
    currentPath.value = props.initialPath || ''
    workspaceName.value = props.initialName || ''
    loadPath(currentPath.value)
  }
})

function navigateTo(path: string) {
  loadPath(path)
}

function selectDirectory(dir: { name: string, path: string }) {
  currentPath.value = dir.path
  if (!props.isUpdate) {
    workspaceName.value = dir.name
  }
  loadPath(dir.path)
}

const breadcrumbs = computed(() => {
  const segments = currentPath.value.split('/').filter(Boolean)
  const crumbs = [{ label: rootName.value, path: '' }]

  let p = ''
  for (const seg of segments) {
    p = p ? `${p}/${seg}` : seg
    crumbs.push({ label: seg, path: p })
  }
  return crumbs
})

function confirm() {
  if (!workspaceName.value.trim()) {
    toast.add({ title: 'Workspace name is required', color: 'error' })
    return
  }
  emit('select', { name: workspaceName.value.trim(), path: currentPath.value })
}
</script>

<template>
  <UModal
    v-model:open="isOpen"
    :title="isUpdate ? 'Confirm Workspace Folder' : 'New Workspace'"
  >
    <template #body>
      <div class="flex flex-col gap-4">
        <UInput
          v-model="workspaceName"
          placeholder="Workspace name..."
          autofocus
        />

        <div class="border border-gray-200 dark:border-gray-800 rounded-md overflow-hidden">
          <div class="bg-gray-50 dark:bg-gray-900 px-3 py-2 border-b border-gray-200 dark:border-gray-800 flex items-center gap-1 text-sm overflow-x-auto">
            <template
              v-for="(crumb, idx) in breadcrumbs"
              :key="crumb.path"
            >
              <span
                v-if="idx > 0"
                class="text-gray-400"
              >/</span>
              <button
                class="hover:text-primary-500 hover:underline px-1 rounded transition-colors"
                @click="navigateTo(crumb.path)"
              >
                {{ crumb.label }}
              </button>
            </template>
          </div>

          <div class="max-h-60 overflow-y-auto p-2">
            <div
              v-if="loading"
              class="flex justify-center p-4"
            >
              <UIcon
                name="i-lucide-loader-2"
                class="animate-spin w-5 h-5 text-gray-500"
              />
            </div>
            <div
              v-else-if="directories.length === 0"
              class="text-center text-sm text-gray-500 p-4"
            >
              No subdirectories found.
            </div>
            <div
              v-else
              class="flex flex-col gap-1"
            >
              <button
                v-for="dir in directories"
                :key="dir.path"
                class="flex items-center gap-2 px-2 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-800 rounded text-left text-sm"
                @click="selectDirectory(dir)"
              >
                <UIcon
                  name="i-lucide-folder"
                  class="w-4 h-4 text-primary-500"
                />
                {{ dir.name }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </template>

    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton
          label="Cancel"
          color="neutral"
          variant="ghost"
          :disabled="pending"
          @click="isOpen = false"
        />
        <UButton
          label="Select this folder"
          :loading="pending"
          @click="confirm"
        />
      </div>
    </template>
  </UModal>
</template>
