<script setup lang="ts">
import type { McpOAuthDiscovery } from '#shared/types/chat'

const props = defineProps<{
  discovery: McpOAuthDiscovery | null
}>()

const registrationTitle = computed(() => {
  if (props.discovery?.registrationMethods.includes('cimd')) return 'Client Identifier Metadata Document (CIMD)'
  if (props.discovery?.registrationMethods.includes('dcr')) return 'Dynamic Client Registration (DCR)'
  return 'No advertised registration method'
})

const registrationDescription = computed(() => {
  if (props.discovery?.registrationMethods.includes('cimd')) return 'The authorization server advertises URL-based client metadata support.'
  if (props.discovery?.registrationMethods.includes('dcr')) return 'The authorization server advertises a Dynamic Client Registration endpoint.'
  return 'The authorization server did not advertise CIMD or Dynamic Client Registration.'
})

const endpoints = computed(() => [
  ['Auth URL', props.discovery?.authorizationUrl],
  ['Token URL', props.discovery?.tokenUrl],
  ['Registration URL', props.discovery?.registrationUrl],
  ['Authorization server', props.discovery?.authorizationServer],
  ['Resource', props.discovery?.resource]
])
</script>

<template>
  <div class="space-y-5 border-t border-default p-4">
    <div>
      <p class="text-xs font-semibold uppercase tracking-wide text-dimmed">
        Client registration
      </p>
      <div class="mt-2 rounded-lg bg-elevated p-3">
        <p class="text-sm font-medium text-highlighted">
          {{ registrationTitle }}
        </p>
        <p class="mt-1 text-xs leading-5 text-muted">
          {{ registrationDescription }}
        </p>
      </div>
    </div>

    <div>
      <p class="text-xs font-semibold uppercase tracking-wide text-dimmed">
        Scopes
      </p>
      <div class="mt-2 rounded-lg bg-elevated p-3">
        <p class="text-xs text-muted">
          Default scopes
        </p>
        <div class="mt-2 flex flex-wrap gap-1.5">
          <UBadge
            v-for="scope in discovery?.scopes ?? []"
            :key="scope"
            :label="scope"
            color="neutral"
            variant="outline"
          />
          <span
            v-if="!(discovery?.scopes.length)"
            class="text-xs text-dimmed"
          >No scopes advertised</span>
        </div>
      </div>
    </div>

    <div>
      <p class="text-xs font-semibold uppercase tracking-wide text-dimmed">
        OAuth endpoints
      </p>
      <dl class="mt-2 space-y-3 rounded-lg bg-elevated p-3">
        <div
          v-for="row in endpoints"
          :key="String(row[0])"
        >
          <dt class="text-xs font-medium text-muted">
            {{ row[0] }}
          </dt>
          <dd class="mt-1 break-all rounded-md bg-default px-2.5 py-2 font-mono text-xs text-highlighted">
            {{ row[1] || 'Not advertised' }}
          </dd>
        </div>
      </dl>
    </div>

    <div v-if="discovery?.oidcEnabled">
      <p class="text-xs font-semibold uppercase tracking-wide text-dimmed">
        OpenID Connect
      </p>
      <div class="mt-2 rounded-lg bg-elevated p-3">
        <div class="flex items-center gap-2 text-sm font-medium text-highlighted">
          <UIcon
            name="i-lucide-circle-check"
            class="size-4 text-success"
          />
          OIDC enabled
        </div>
        <p class="mt-2 break-all font-mono text-xs text-muted">
          {{ discovery.oidcConfigurationUrl || 'Discovery URL not available' }}
        </p>
      </div>
    </div>
  </div>
</template>
