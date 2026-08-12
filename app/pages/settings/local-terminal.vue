<script setup lang="ts">
import type { RelayExecResult } from '~/composables/useRelayAgent'

useSeoMeta({ title: 'Local Terminal' })

const {
  port,
  isConnected,
  isConnecting,
  error: _error,
  checkConnection,
  exec
} = useRelayAgent()

const commandInput = ref('')
const execPending = ref(false)
const history = ref<Array<{ id: string, command: string, result: RelayExecResult }>>([])

// The relay runs as a separate local process, so it cannot read this app's
// runtime config. Pass the origin this page is actually served from so the
// relay's local Origin policy can validate browser requests correctly.
const siteOrigin = useRequestURL().origin
const relayBinaryName = 'relay-agent-x86_64-unknown-linux-gnu'
const relayDownloadUrl = `https://github.com/farismnrr/ai-code/releases/latest/download/${relayBinaryName}`

onMounted(() => {
  if (!isConnected.value) {
    void checkConnection()
  }
})

async function handleExec() {
  const cmd = commandInput.value.trim()
  if (!cmd || execPending.value) return

  execPending.value = true
  const entryId = Math.random().toString(36).substring(2, 9)

  try {
    const res = await exec(cmd)
    history.value.push({ id: entryId, command: cmd, result: res })
    commandInput.value = ''
  } catch (err: unknown) {
    toast.add({ title: 'Execution failed', description: (err as Error).message, color: 'error' })
  } finally {
    execPending.value = false
  }
}
</script>

<template>
  <div class="space-y-6 py-4">
    <div>
      <h2 class="text-base font-semibold text-highlighted">
        Local MCP Relay
      </h2>
      <p class="text-sm text-muted">
        Connect directly to the Rust relay running on your Linux machine. The
        local relay exposes MCP on loopback and executes commands inside its
        configured Bubblewrap filesystem boundary.
      </p>
    </div>

    <UCard>
      <template #header>
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <UIcon
              name="i-lucide-laptop"
              class="size-5 text-highlighted"
            />
            <span class="font-medium text-highlighted">Local relay status</span>
          </div>
          <UBadge
            :label="isConnected ? 'Connected' : 'Disconnected'"
            :color="isConnected ? 'success' : 'neutral'"
            variant="subtle"
          />
        </div>
      </template>

      <div
        v-if="isConnected"
        class="space-y-4"
      >
        <div class="flex items-center justify-between text-sm">
          <span class="text-muted">Target host</span>
          <code class="font-mono text-highlighted">http://127.0.0.1:{{ port }}</code>
        </div>
        <div class="flex gap-2">
          <UButton
            label="Refresh connection"
            icon="i-lucide-refresh-cw"
            :loading="isConnecting"
            color="primary"
            variant="outline"
            @click="checkConnection"
          />
        </div>
      </div>

      <div
        v-else
        class="space-y-4"
      >
        <div class="rounded-lg border border-default bg-elevated/50 p-4 space-y-3">
          <h3 class="flex items-center gap-2 text-sm font-medium text-highlighted">
            <UIcon
              name="i-lucide-download"
              class="size-4"
            />
            1. Install the Linux relay
          </h3>
          <p class="text-xs text-muted">
            The current relay release target is Linux x86_64 only. Bubblewrap
            (<code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">bwrap</code>)
            must be installed, and the relay refuses to run as root.
          </p>

          <UButton
            label="Download Linux x86_64 relay"
            icon="i-lucide-download"
            color="neutral"
            variant="outline"
            size="xs"
            :to="relayDownloadUrl"
            target="_blank"
          />
        </div>

        <div class="space-y-3">
          <h3 class="flex items-center gap-2 text-sm font-medium text-highlighted">
            <UIcon
              name="i-lucide-terminal"
              class="size-4"
            />
            2. Run the relay
          </h3>

          <p class="text-xs text-muted">
            Pick the project directory the relay is allowed to access. The
            <code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">--execution-root</code>
            is the containment boundary; commands cannot freely access the rest
            of your machine. <code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">--dir</code>
            sets the default working directory inside that boundary.
          </p>

          <pre class="overflow-x-auto rounded-lg border border-default bg-elevated p-3 text-xs text-highlighted"><code>chmod +x ./{{ relayBinaryName }}
./{{ relayBinaryName }} \
  --mode local \
  --dir /path/to/project \
  --execution-root /path/to/project \
  --origin {{ siteOrigin }}</code></pre>

          <p class="text-xs text-muted">
            The default port is <code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">47821</code>.
            Add <code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">--port N</code>
            only when using a different local port. Stop a foreground relay with
            <code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">Ctrl+C</code>.
          </p>

          <p class="text-xs text-muted">
            <span class="font-medium text-highlighted">Background example:</span>
          </p>
          <pre class="overflow-x-auto rounded-lg border border-default bg-elevated p-3 text-xs text-highlighted"><code>nohup ./{{ relayBinaryName }} \
  --mode local \
  --dir /path/to/project \
  --execution-root /path/to/project \
  --origin {{ siteOrigin }} \
  &gt; relay-agent.log 2&gt;&amp;1 &amp; disown</code></pre>

          <div class="flex gap-2 mt-4">
            <UButton
              label="Check connection"
              icon="i-lucide-refresh-cw"
              :loading="isConnecting"
              @click="checkConnection"
            />
          </div>
        </div>
      </div>
    </UCard>

    <UCard
      v-if="isConnected"
      class="space-y-4"
    >
      <template #header>
        <div class="flex items-center justify-between">
          <span class="font-medium text-highlighted">Local terminal</span>
          <UButton
            v-if="history.length"
            label="Clear output"
            color="neutral"
            variant="ghost"
            size="xs"
            @click="history = []"
          />
        </div>
      </template>

      <div class="min-h-48 max-h-96 overflow-y-auto rounded-lg border border-default bg-elevated p-4 font-mono text-xs space-y-3">
        <div
          v-if="!history.length"
          class="italic text-dimmed"
        >
          Terminal ready. Commands execute inside the relay's configured execution root.
        </div>

        <div
          v-for="item in history"
          :key="item.id"
          class="space-y-1"
        >
          <div class="flex items-center gap-2 text-highlighted">
            <span class="text-primary">$</span>
            <span class="font-semibold">{{ item.command }}</span>
          </div>

          <pre
            v-if="item.result.stdout"
            class="whitespace-pre-wrap text-muted"
          >{{ item.result.stdout }}</pre>
          <pre
            v-if="item.result.stderr"
            class="whitespace-pre-wrap text-error"
          >{{ item.result.stderr }}</pre>
          <div
            v-if="item.result.error"
            class="text-error"
          >
            Error: {{ item.result.error }}
          </div>
        </div>
      </div>

      <div class="flex gap-2">
        <UInput
          v-model="commandInput"
          placeholder="Run command (e.g. ls, pwd, git status)..."
          class="flex-1 font-mono"
          :disabled="execPending"
          @keyup.enter="handleExec"
        />
        <UButton
          label="Run"
          icon="i-lucide-play"
          :loading="execPending"
          @click="handleExec"
        />
      </div>
    </UCard>
  </div>
</template>
