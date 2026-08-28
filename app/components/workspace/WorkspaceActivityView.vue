<script setup lang="ts">
import type { ActivityDetail, ActivityDiff, ActivityItem, ActivityResponse } from '#shared/types/activity'

const props = defineProps<{ workspaceId: string, initialData: ActivityResponse, initialStatus: string, initialError: unknown }>()
const emit = defineEmits<{ refresh: [] }>()
const route = useRoute()
const router = useRouter()
const items = ref<ActivityItem[]>([...props.initialData.items])
const nextCursor = ref(props.initialData.nextCursor)
const hasMore = ref(props.initialData.hasMore)
const loadingOlder = ref(false)
const selected = ref<ActivityItem | null>(null)
const diff = ref<ActivityDiff | null>(null)
const diffError = ref(false)
const detailLoading = ref(false)
const clearOpen = ref(false)
const clearLoading = ref(false)
const live = ref(true)
const degraded = ref(Boolean(props.initialData.degraded))
const filters = reactive({ text: String(route.query.q ?? ''), category: String(route.query.category ?? 'all'), status: String(route.query.status ?? 'all') })
const categories = computed<string[]>(() => ['all', ...new Set(items.value.map(item => item.category))])
const visibleItems = computed(() => items.value.filter((item) => {
  const text = filters.text.trim().toLowerCase()
  return (!text || [item.action, item.operation, item.target, item.result, item.actor?.label, item.actor?.source, item.clientInfo?.name, item.clientInfo?.version, ...(item.affectedPaths ?? [])].some(value => value?.toLowerCase().includes(text))) && (filters.category === 'all' || item.category === filters.category) && (filters.status === 'all' || item.status === filters.status)
}))
let pollTimer: ReturnType<typeof setInterval> | undefined
let queryTimer: ReturnType<typeof setTimeout> | undefined

function displayAction(item: ActivityItem) {
  if (item.action) return item.action
  return `${item.operation} · input not recorded`
}

function displayResult(item: ActivityItem) {
  if (item.result && !['operation admitted', 'tool execution completed', 'tool execution failed'].includes(item.result)) return item.result
  if (item.status === 'ok') return 'completed'
  if (item.status === 'error') return 'failed'
  if (item.status === 'denied') return 'denied'
  if (item.status === 'cancelled') return 'cancelled'
  if (item.status === 'interrupted') return 'interrupted'
  return item.status
}

function displayResultDetail(item: ActivityItem) {
  return item.resultDetail || item.result || displayResult(item)
}

function queryUpdate() {
  void router.replace({ query: { ...route.query, q: filters.text || undefined, category: filters.category === 'all' ? undefined : filters.category, status: filters.status === 'all' ? undefined : filters.status } })
  void reloadFilter()
}
async function reloadFilter() {
  try {
    const response = await $fetch<ActivityResponse>(`/api/workspaces/${props.workspaceId}/activity`, { query: { limit: 30, q: filters.text, category: filters.category === 'all' ? undefined : filters.category, status: filters.status === 'all' ? undefined : filters.status } })
    items.value = response.items
    nextCursor.value = response.nextCursor
    hasMore.value = response.hasMore
    degraded.value = Boolean(response.degraded)
  } catch {
    degraded.value = true
  }
}
async function loadOlder() {
  if (!nextCursor.value || loadingOlder.value) return
  loadingOlder.value = true
  try {
    const response = await $fetch<ActivityResponse>(`/api/workspaces/${props.workspaceId}/activity`, { query: { limit: 30, cursor: nextCursor.value, q: filters.text, category: filters.category === 'all' ? undefined : filters.category, status: filters.status === 'all' ? undefined : filters.status } })
    const known = new Set(items.value.map(item => item.id))
    items.value.push(...response.items.filter(item => !known.has(item.id)))
    nextCursor.value = response.nextCursor
    hasMore.value = response.hasMore
  } catch {
    // Retain the current page when an older cursor request fails.
  } finally {
    loadingOlder.value = false
  }
}
async function poll() {
  if (!live.value || (import.meta.client && document.hidden)) return
  try {
    const response = await $fetch<ActivityResponse>(`/api/workspaces/${props.workspaceId}/activity`, { query: { limit: 30, since: items.value[0]?.occurredAt } })
    const byId = new Map(items.value.map(item => [item.id, item]))
    for (const item of response.items) {
      const old = byId.get(item.id)
      if (!old || old.status === 'running' || old.status === 'started') byId.set(item.id, item)
    }
    items.value = [...byId.values()].sort((a, b) => b.occurredAt.localeCompare(a.occurredAt))
    degraded.value = Boolean(response.degraded)
  } catch { degraded.value = true }
}
async function openDetail(item: ActivityItem) {
  selected.value = item
  detailLoading.value = true
  diff.value = null
  diffError.value = false
  try {
    selected.value = await $fetch<ActivityDetail>(`/api/workspaces/${props.workspaceId}/activity/${item.id}`)
  } catch {
    // The summary remains useful when the lazy detail request is unavailable.
  } finally {
    detailLoading.value = false
  }
}
async function loadDiff() {
  if (!selected.value || selected.value.evidence !== 'exact') return
  diffError.value = false
  try {
    diff.value = await $fetch<ActivityDiff>(`/api/workspaces/${props.workspaceId}/activity/${selected.value.id}/diff`)
  } catch {
    diffError.value = true
  }
}
async function clearHistory() {
  clearLoading.value = true
  try {
    await $fetch(`/api/workspaces/${props.workspaceId}/activity`, { method: 'DELETE' } as never)
    items.value = []
    nextCursor.value = null
    hasMore.value = false
    clearOpen.value = false
  } finally { clearLoading.value = false }
}
onMounted(() => {
  pollTimer = setInterval(poll, 15000)
})
onBeforeUnmount(() => {
  if (pollTimer) clearInterval(pollTimer)
  if (queryTimer) clearTimeout(queryTimer)
})
watch(() => [filters.text, filters.category, filters.status], () => {
  nextCursor.value = null
  hasMore.value = false
  if (queryTimer) clearTimeout(queryTimer)
  queryTimer = setTimeout(queryUpdate, 250)
})
watch(() => props.initialData, (data) => {
  items.value = [...data.items]
  nextCursor.value = data.nextCursor
  hasMore.value = data.hasMore
  degraded.value = Boolean(data.degraded)
}, { deep: false })
</script>

<template>
  <div class="mx-auto flex w-full max-w-5xl flex-col gap-5 py-4 sm:py-6">
    <header class="flex flex-wrap items-start justify-between gap-4">
      <div>
        <h1 class="text-xl font-semibold text-highlighted">
          Activity
        </h1><p class="text-sm text-muted">
          Durable workspace actions and change evidence.
        </p>
      </div>
      <div class="flex gap-2">
        <UButton
          :icon="live ? 'i-lucide-pause' : 'i-lucide-play'"
          :label="live ? 'Live' : 'Paused'"
          color="neutral"
          variant="soft"
          @click="live = !live"
        /><UButton
          icon="i-lucide-refresh-cw"
          label="Refresh"
          color="neutral"
          variant="outline"
          @click="emit('refresh'); poll()"
        /><UButton
          icon="i-lucide-trash-2"
          label="Clear history"
          color="error"
          variant="ghost"
          @click="clearOpen = true"
        />
      </div>
    </header>
    <UAlert
      v-if="degraded"
      icon="i-lucide-cloud-off"
      color="warning"
      title="Live updates are delayed"
      description="Your durable history is still available. We will retry while this page is open."
    />
    <div class="grid gap-3 sm:grid-cols-[1fr_10rem_10rem]">
      <UInput
        v-model="filters.text"
        icon="i-lucide-search"
        placeholder="Filter paths, actions, or actors"
        aria-label="Filter activity"
      />
      <USelect
        v-model="filters.category"
        :items="categories"
        aria-label="Filter category"
      />
      <USelect
        v-model="filters.status"
        :items="['all', 'running', 'ok', 'error', 'denied', 'cancelled', 'interrupted']"
        aria-label="Filter status"
      />
    </div>
    <div
      v-if="initialStatus === 'pending'"
      class="space-y-3"
    >
      <USkeleton
        v-for="i in 5"
        :key="i"
        class="h-20 w-full rounded-xl"
      />
    </div>
    <DataLoadError
      v-else-if="initialError"
      title="Couldn't load workspace logs"
      description="Retry to request your activity history."
      @retry="emit('refresh')"
    />
    <UCard v-else-if="!visibleItems.length">
      <div class="py-12 text-center">
        <UIcon
          name="i-lucide-activity"
          class="mx-auto mb-3 size-8 text-dimmed"
        /><p class="font-medium text-highlighted">
          {{ items.length ? 'No activity matches these filters' : 'No workspace activity yet' }}
        </p><p class="mt-1 text-sm text-muted">
          Actions recorded for this workspace will appear here.
        </p>
      </div>
    </UCard>
    <div
      v-else
      class="space-y-3"
      aria-live="polite"
    >
      <UCard
        v-for="item in visibleItems"
        :key="item.id"
        class="transition hover:border-primary/40"
      >
        <button
          class="flex w-full items-start gap-3 text-left"
          :aria-label="`Inspect ${item.operation} activity`"
          @click="openDetail(item)"
        >
          <UIcon
            :name="item.status === 'running' ? 'i-lucide-loader-circle' : item.status === 'ok' ? 'i-lucide-check-circle' : 'i-lucide-circle-alert'"
            class="mt-0.5 size-5 shrink-0"
            :class="item.status === 'running' ? 'animate-spin text-primary' : 'text-muted'"
          /><span class="min-w-0 flex-1">
            <span class="flex flex-wrap items-center gap-x-2 gap-y-1">
              <strong class="break-words text-sm text-highlighted">{{ displayAction(item) }}</strong>
              <span class="rounded-full bg-elevated px-2 py-0.5 text-xs text-muted">{{ item.status }}</span>
            </span>
            <span class="mt-1 block truncate font-mono text-sm text-muted">{{ displayResult(item) }}</span>
            <span class="mt-2 flex flex-wrap gap-3 text-xs text-dimmed">
              <span>{{ new Date(item.occurredAt).toLocaleString() }}</span>
              <span v-if="item.durationMs !== undefined">{{ item.durationMs }} ms</span>
              <span v-if="item.affectedPaths?.length">{{ item.affectedPaths.length }} file{{ item.affectedPaths.length === 1 ? '' : 's' }}</span>
              <span
                v-if="item.additions || item.deletions"
                class="text-primary"
              >+{{ item.additions || 0 }} / -{{ item.deletions || 0 }}</span>
            </span>
          </span><UIcon
            name="i-lucide-chevron-right"
            class="size-4 text-dimmed"
          />
        </button>
      </UCard>
    </div>
    <UButton
      v-if="hasMore"
      :loading="loadingOlder"
      label="Load older activity"
      color="neutral"
      variant="outline"
      class="self-center"
      @click="loadOlder"
    />
  </div>
  <USlideover
    :open="selected !== null"
    title="Activity details"
    @update:open="(open) => { if (!open) selected = null }"
  >
    <template #body>
      <div
        v-if="detailLoading"
        class="space-y-3"
      >
        <USkeleton class="h-5 w-1/2" /><USkeleton class="h-24 w-full" />
      </div><div
        v-else-if="selected"
        class="space-y-4 text-sm"
      >
        <section class="space-y-2">
          <p class="text-xs font-medium uppercase tracking-wide text-dimmed">
            Action
          </p>
          <pre class="whitespace-pre-wrap break-words rounded-lg bg-elevated p-3 font-mono text-sm leading-6 text-highlighted">{{ displayAction(selected) }}</pre>
        </section>
        <section class="space-y-2">
          <div class="flex items-center justify-between gap-3">
            <p class="text-xs font-medium uppercase tracking-wide text-dimmed">
              Result
            </p>
            <span class="rounded-full bg-elevated px-2 py-0.5 text-xs text-muted">{{ selected.status }}</span>
          </div>
          <pre class="max-h-[28rem] overflow-auto whitespace-pre-wrap break-words rounded-lg bg-elevated p-3 font-mono text-sm leading-6 text-highlighted">{{ displayResultDetail(selected) }}</pre>
          <p class="text-xs text-muted">
            {{ new Date(selected.occurredAt).toLocaleString() }}<span v-if="selected.durationMs !== undefined"> · {{ selected.durationMs }} ms</span>
          </p>
        </section>
        <section
          v-if="selected.affectedPaths?.length"
          class="space-y-2"
        >
          <p class="text-xs font-medium uppercase tracking-wide text-dimmed">
            Files changed
          </p>
          <div class="rounded-lg bg-elevated p-3">
            <p
              v-for="path in selected.affectedPaths"
              :key="path"
              class="break-all font-mono text-xs text-highlighted"
            >
              {{ path }}
            </p>
            <p
              v-if="selected.additions || selected.deletions"
              class="mt-2 text-xs text-primary"
            >
              +{{ selected.additions || 0 }} / -{{ selected.deletions || 0 }} lines
            </p>
          </div>
        </section>
        <UButton
          v-if="selected.evidence === 'exact'"
          label="Load historical diff"
          icon="i-lucide-file-diff"
          color="neutral"
          variant="outline"
          @click="loadDiff"
        /><UAlert
          v-if="diffError"
          color="warning"
          title="Historical diff unavailable"
          description="The encrypted evidence could not be loaded right now."
        /><div
          v-if="diff"
          class="space-y-3"
        >
          <p class="text-xs text-muted">
            {{ diff.complete === false ? 'This diff is incomplete.' : 'Exact historical diff' }}
          </p><pre
            v-for="file in diff.files"
            :key="file.path"
            class="overflow-x-auto rounded-lg bg-elevated p-3 text-xs leading-5"
          ><code><span class="text-muted">{{ file.path }} (+{{ file.additions || 0 }} / -{{ file.deletions || 0 }})</span>{{ '\n' }}<span
            v-for="(hunk, index) in file.hunks"
            :key="index"
          >{{ hunk }}{{ '\n' }}</span></code></pre>
        </div>
        <details class="rounded-lg border border-default p-3 text-xs text-muted">
          <summary class="cursor-pointer select-none font-medium text-highlighted">
            Technical details
          </summary>
          <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-3 gap-y-2">
            <dt>
              Tool
            </dt><dd class="font-mono">
              {{ selected.operation }}
            </dd>
            <dt>
              Client
            </dt><dd>
              {{ selected.clientInfo ? `${selected.clientInfo.name} ${selected.clientInfo.version}` : (selected.actor?.label || 'External MCP client') }}
            </dd>
            <dt v-if="selected.target">
              Target
            </dt><dd
              v-if="selected.target"
              class="break-all font-mono"
            >
              {{ selected.target }}
            </dd>
            <dt>
              Evidence
            </dt><dd>
              {{ selected.evidence.replace('_', ' ') }}
            </dd>
          </dl>
        </details>
      </div>
    </template>
  </USlideover>
  <UModal
    v-model:open="clearOpen"
    title="Clear workspace history"
  >
    <template #body>
      <p class="text-sm text-muted">
        This permanently removes retained activity for this workspace. Future activity will continue to be recorded; relay access is not revoked.
      </p>
    </template><template #footer>
      <div class="flex justify-end gap-2">
        <UButton
          label="Cancel"
          color="neutral"
          variant="ghost"
          @click="clearOpen = false"
        /><UButton
          label="Clear history"
          color="error"
          :loading="clearLoading"
          @click="clearHistory"
        />
      </div>
    </template>
  </UModal>
</template>
