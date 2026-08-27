<script setup lang="ts">
type ConnectionStatus = 'connected' | 'connecting' | 'disconnected' | 'error' | 'unsupported'

const props = defineProps<{
  name: string
  description: string
  kind: string
  status: ConnectionStatus
  endpoint?: string
  toolCount?: number
  icon?: string
}>()

const STATUS_COLOR = {
  connected: 'success',
  connecting: 'neutral',
  disconnected: 'neutral',
  error: 'error',
  unsupported: 'warning'
} as const

const statusLabel = computed(() => ({
  connected: 'Connected',
  connecting: 'Checking',
  disconnected: 'Disconnected',
  error: 'Needs attention',
  unsupported: 'Unsupported'
}[props.status]))

const statusColor = computed(() => STATUS_COLOR[props.status])
</script>

<template>
  <UCard :ui="{ body: 'space-y-4' }">
    <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
      <div class="flex min-w-0 gap-3">
        <div class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-elevated">
          <UIcon
            :name="icon ?? 'i-lucide-blocks'"
            class="size-5 text-muted"
          />
        </div>
        <div class="min-w-0">
          <div class="flex flex-wrap items-center gap-2">
            <h3 class="font-medium text-highlighted">
              {{ name }}
            </h3>
            <UBadge
              :label="statusLabel"
              :color="statusColor"
              variant="subtle"
              size="sm"
            />
            <UBadge
              :label="kind"
              color="neutral"
              variant="outline"
              size="sm"
            />
          </div>
          <p class="mt-1 text-sm text-muted">
            {{ description }}
          </p>
          <div class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-dimmed">
            <code
              v-if="endpoint"
              class="break-all font-mono"
            >{{ endpoint }}</code>
            <span v-if="toolCount !== undefined">
              {{ toolCount }} {{ toolCount === 1 ? 'tool' : 'tools' }}
            </span>
          </div>
        </div>
      </div>

      <div class="flex shrink-0 flex-wrap items-center gap-2 sm:justify-end">
        <slot name="actions" />
      </div>
    </div>

    <slot />
  </UCard>
</template>
