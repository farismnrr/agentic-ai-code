<script setup lang="ts">
import { buildLocalRelayCommand, LOCAL_RELAY_BINARY, LOCAL_RELAY_DOWNLOAD_URL } from '#shared/utils/local-relay'

const { port, isConnected, isConnecting, error, checkConnection } = useRelayAgent()
const toast = useToast()
const allowTerminalNetwork = ref(false)
const siteOrigin = useRequestURL().origin

const foregroundCommand = computed(() => buildLocalRelayCommand({
  origin: siteOrigin,
  port: port.value,
  allowTerminalNetwork: allowTerminalNetwork.value
}))
const backgroundCommand = computed(() => buildLocalRelayCommand({
  origin: siteOrigin,
  port: port.value,
  allowTerminalNetwork: allowTerminalNetwork.value,
  background: true
}))

onMounted(() => {
  if (!isConnected.value) void checkConnection()
})

async function copyCommand(command: string) {
  await navigator.clipboard.writeText(command)
  toast.add({ title: 'Command copied', icon: 'i-lucide-check', color: 'success' })
}
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-col gap-3 rounded-lg border border-default bg-elevated/40 p-4 sm:flex-row sm:items-center sm:justify-between">
      <div>
        <div class="flex items-center gap-2">
          <UIcon
            name="i-lucide-laptop"
            class="size-4 text-muted"
          />
          <p class="text-sm font-medium text-highlighted">
            Local relay
          </p>
          <UBadge
            :label="isConnecting ? 'Checking' : isConnected ? 'Connected' : 'Disconnected'"
            :color="isConnecting ? 'neutral' : isConnected ? 'success' : 'neutral'"
            variant="subtle"
            size="sm"
          />
        </div>
        <p class="mt-1 text-xs text-muted">
          Browser-local MCP at <code class="font-mono text-highlighted">127.0.0.1:{{ port }}</code>. It is never stored as a remote server.
        </p>
      </div>
      <UButton
        label="Check connection"
        icon="i-lucide-refresh-cw"
        color="neutral"
        variant="outline"
        :loading="isConnecting"
        @click="checkConnection"
      />
    </div>

    <ol class="space-y-4">
      <li class="rounded-lg border border-default p-4">
        <div class="flex gap-3">
          <div class="flex size-7 shrink-0 items-center justify-center rounded-full bg-elevated text-xs font-semibold text-muted">
            1
          </div>
          <div class="min-w-0 flex-1 space-y-3">
            <div>
              <h3 class="text-sm font-medium text-highlighted">
                Install relay
              </h3>
              <p class="mt-1 text-xs leading-5 text-muted">
                Linux x86_64 only. Bubblewrap (<code class="font-mono text-highlighted">bwrap</code>) must be installed, and the relay refuses to run as root.
              </p>
            </div>
            <UButton
              label="Download Linux relay"
              icon="i-lucide-download"
              color="neutral"
              variant="outline"
              size="sm"
              :to="LOCAL_RELAY_DOWNLOAD_URL"
              target="_blank"
            />
          </div>
        </div>
      </li>

      <li class="rounded-lg border border-default p-4">
        <div class="flex gap-3">
          <div class="flex size-7 shrink-0 items-center justify-center rounded-full bg-elevated text-xs font-semibold text-muted">
            2
          </div>
          <div class="min-w-0 flex-1 space-y-4">
            <div>
              <h3 class="text-sm font-medium text-highlighted">
                Start relay
              </h3>
              <p class="mt-1 text-xs leading-5 text-muted">
                Make <code class="font-mono text-highlighted">{{ LOCAL_RELAY_BINARY }}</code> executable, then start it from your non-root account. The execution root is the filesystem ceiling; the working directory can switch inside it.
              </p>
            </div>

            <div class="flex items-start gap-3 rounded-lg bg-elevated/60 p-3">
              <USwitch
                v-model="allowTerminalNetwork"
                size="sm"
                aria-label="Allow terminal network access in generated relay command"
                class="mt-0.5 shrink-0"
              />
              <div>
                <p class="text-xs font-medium text-highlighted">
                  Allow terminal network access
                </p>
                <p class="mt-1 text-xs leading-5 text-muted">
                  Adds <code class="font-mono text-highlighted">--allow-terminal-network</code> to the launch command. This only changes the generated command; restart an already-running relay to apply it.
                </p>
              </div>
            </div>

            <div class="space-y-2">
              <div class="flex items-center justify-between gap-3">
                <p class="text-xs font-medium text-muted">
                  Foreground
                </p>
                <UButton
                  label="Copy"
                  icon="i-lucide-copy"
                  color="neutral"
                  variant="ghost"
                  size="xs"
                  @click="copyCommand(foregroundCommand)"
                />
              </div>
              <pre class="overflow-x-auto rounded-lg border border-default bg-elevated p-3 text-xs leading-5 text-highlighted"><code>chmod +x ./{{ LOCAL_RELAY_BINARY }}
              {{ foregroundCommand }}</code></pre>
            </div>

            <UCollapsible>
              <UButton
                label="Run in background"
                icon="i-lucide-chevron-down"
                color="neutral"
                variant="ghost"
                size="xs"
              />
              <template #content>
                <div class="space-y-2 pt-2">
                  <div class="flex justify-end">
                    <UButton
                      label="Copy"
                      icon="i-lucide-copy"
                      color="neutral"
                      variant="ghost"
                      size="xs"
                      @click="copyCommand(backgroundCommand)"
                    />
                  </div>
                  <pre class="overflow-x-auto rounded-lg border border-default bg-elevated p-3 text-xs leading-5 text-highlighted"><code>{{ backgroundCommand }}</code></pre>
                </div>
              </template>
            </UCollapsible>
          </div>
        </div>
      </li>

      <li class="rounded-lg border border-default p-4">
        <div class="flex gap-3">
          <div class="flex size-7 shrink-0 items-center justify-center rounded-full bg-elevated text-xs font-semibold text-muted">
            3
          </div>
          <div class="min-w-0 flex-1">
            <h3 class="text-sm font-medium text-highlighted">
              Verify connection
            </h3>
            <p class="mt-1 text-xs leading-5 text-muted">
              AI Code verifies the relay's MCP protocol before enabling Agent Mode.
            </p>
            <div class="mt-3 flex flex-wrap items-center gap-3">
              <UButton
                :label="isConnected ? 'Recheck relay' : 'Verify relay'"
                icon="i-lucide-plug-zap"
                :loading="isConnecting"
                @click="checkConnection"
              />
              <p
                v-if="!isConnected && error"
                class="text-xs text-muted"
              >
                {{ error }}
              </p>
            </div>
          </div>
        </div>
      </li>
    </ol>
  </div>
</template>
