<script setup lang="ts">
import type { RelayExecResult } from '~/composables/useRelayAgent'

useSeoMeta({ title: 'Local Terminal' })

const {
  port,
  isConnected,
  isConnecting,
  error,
  checkConnection,
  exec
} = useRelayAgent()


const commandInput = ref('')
const execPending = ref(false)
const history = ref<Array<{ id: string, command: string, result: RelayExecResult }>>([])

const detectedOs = ref<'linux' | 'macos-arm64' | 'macos-x64' | 'windows' | 'unknown'>('linux')

// The relay-agent CLI runs on the user's own machine as its own process —
// it cannot read this app's runtime config, so its `--origin` default is
// only ever a local-dev convenience (see packages/relay-agent/src/server.ts).
// Whatever origin this page is actually being viewed from (dev, staging,
// the real Singapore-hosted production domain) must be passed explicitly,
// so show it here rather than let the user guess or copy a stale example.
// `useRequestURL()` (not `window.location`) works during SSR too, so this
// is correct on first paint, not just after hydration.
const siteOrigin = useRequestURL().origin

onMounted(() => {
  if (!isConnected.value) {
    void checkConnection()
  }

  if (import.meta.client) {
    const ua = navigator.userAgent.toLowerCase()
    const platform = navigator.platform.toLowerCase()

    if (platform.includes('win')) {
      detectedOs.value = 'windows'
    } else if (platform.includes('mac') || ua.includes('macintosh')) {
      // Basic arm64 check for Apple Silicon vs Intel
      if (navigator.maxTouchPoints > 0 || ua.includes('arm64')) {
        detectedOs.value = 'macos-arm64'
      } else {
        detectedOs.value = 'macos-x64'
      }
    } else if (platform.includes('linux')) {
      detectedOs.value = 'linux'
    }
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



const downloadLinks = {
  'linux': 'https://github.com/farismnrr/ai-code/releases/latest/download/relay-agent-linux-x64',
  'macos-arm64': 'https://github.com/farismnrr/ai-code/releases/latest/download/relay-agent-macos-arm64',
  'macos-x64': 'https://github.com/farismnrr/ai-code/releases/latest/download/relay-agent-macos-x64',
  'windows': 'https://github.com/farismnrr/ai-code/releases/latest/download/relay-agent-win-x64.exe'
}
</script>

<template>
  <div class="space-y-6 py-4">
    <div>
      <h2 class="text-base font-semibold text-highlighted">
        Local CLI Relay Agent
      </h2>
      <p class="text-sm text-muted">
        Connect directly to a relay agent CLI running on your local machine.
        Terminal data never leaves your computer over the internet. Once
        paired, the AI can also use this automatically in Agent-mode
        conversations — there's no separate toggle for it in the chat Tools
        picker, but every command still requires your approval there before
        it runs.
      </p>
    </div>

    <!-- Connection / Pairing Status Card -->
    <UCard>
      <template #header>
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <UIcon
              name="i-lucide-laptop"
              class="size-5 text-highlighted"
            />
            <span class="font-medium text-highlighted">Local Agent Status</span>
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
          <span class="text-muted">Target Host</span>
          <code class="font-mono text-highlighted">http://127.0.0.1:{{ port }}</code>
        </div>
        <div class="flex gap-2">
          <UButton
            label="Refresh Connection"
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
        <!-- Standalone Binary Download Section -->
        <div class="rounded-lg border border-white/10 bg-elevated/50 p-4 space-y-3">
          <h3 class="flex items-center gap-2 text-sm font-medium text-highlighted">
            <UIcon
              name="i-lucide-download"
              class="size-4"
            />
            1. Download Standalone Executable (No Node.js / npm required)
          </h3>
          <p class="text-xs text-muted">
            Download and run the standalone binary for your platform, or run via <code class="rounded bg-black/40 px-1 py-0.5 font-mono text-highlighted">npx @ai-code/relay-agent start</code>:
          </p>

          <div class="flex flex-wrap gap-2">
            <UButton
              label="Linux (x64)"
              icon="i-lucide-download"
              :color="detectedOs === 'linux' ? 'primary' : 'neutral'"
              :variant="detectedOs === 'linux' ? 'solid' : 'outline'"
              size="xs"
              :to="downloadLinks.linux"
              target="_blank"
            />
            <UButton
              label="macOS (Apple Silicon arm64)"
              icon="i-lucide-download"
              :color="detectedOs === 'macos-arm64' ? 'primary' : 'neutral'"
              :variant="detectedOs === 'macos-arm64' ? 'solid' : 'outline'"
              size="xs"
              :to="downloadLinks['macos-arm64']"
              target="_blank"
            />
            <UButton
              label="macOS (Intel x64)"
              icon="i-lucide-download"
              :color="detectedOs === 'macos-x64' ? 'primary' : 'neutral'"
              :variant="detectedOs === 'macos-x64' ? 'solid' : 'outline'"
              size="xs"
              :to="downloadLinks['macos-x64']"
              target="_blank"
            />
            <UButton
              label="Windows (x64)"
              icon="i-lucide-download"
              :color="detectedOs === 'windows' ? 'primary' : 'neutral'"
              :variant="detectedOs === 'windows' ? 'solid' : 'outline'"
              size="xs"
              :to="downloadLinks.windows"
              target="_blank"
            />
          </div>

          <p class="text-xs text-muted italic">
            Note: On macOS or Windows, if Gatekeeper/SmartScreen blocks unsigned binaries on first run, right-click the binary and select Open, or run <code class="rounded bg-black/40 px-1 font-mono">xattr -d com.apple.quarantine ./relay-agent-macos-*</code>.
          </p>
        </div>

        <div class="space-y-2">
          <h3 class="flex items-center gap-2 text-sm font-medium text-highlighted">
            <UIcon
              name="i-lucide-key"
              class="size-4"
            />
            2. Run Agent
          </h3>
          <p class="text-xs text-muted">
            Run the binary in your terminal, passing this page's own origin so the agent accepts requests from it (e.g. <code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">./relay-agent-linux-x64 --origin {{ siteOrigin }}</code>). This agent has no directory restriction — it can run commands anywhere on this machine your user account can access, not just one project folder (add <code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">--dir ./some/path</code> only to change its default starting directory).
          </p>

          <p class="text-xs text-muted">
            <span class="font-medium text-highlighted">Start:</span> just run the binary as shown above (add <code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">--port N</code> if not using the default port).
            <span class="font-medium text-highlighted">Stop:</span> <code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">Ctrl+C</code> in that terminal.
          </p>

          <p class="text-xs text-muted">
            <span class="font-medium text-highlighted">Run in the background</span> instead of tying up a terminal: <code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">nohup ./relay-agent-linux-x64 --origin {{ siteOrigin }} &gt; relay-agent.log 2&gt;&amp;1 &amp; disown</code>.
          </p>

          <div class="flex gap-2 mt-4">
            <UButton
              label="Check Connection"
              icon="i-lucide-refresh-cw"
              :loading="isConnecting"
              @click="checkConnection"
            />
          </div>
        </div>
      </div>
    </UCard>

    <!-- Terminal Runner Interface -->
    <UCard
      v-if="isConnected"
      class="space-y-4"
    >
      <template #header>
        <div class="flex items-center justify-between">
          <span class="font-medium text-highlighted">Local Terminal</span>
          <UButton
            v-if="history.length"
            label="Clear Output"
            color="neutral"
            variant="ghost"
            size="xs"
            @click="history = []"
          />
        </div>
      </template>

      <!-- Terminal Output Window -->
      <div class="min-h-48 max-h-96 overflow-y-auto rounded-lg border border-white/10 bg-black/90 p-4 font-mono text-xs text-green-400 space-y-3">
        <div
          v-if="!history.length"
          class="italic text-gray-500"
        >
          Terminal ready. Type a command below to execute on this machine.
        </div>

        <div
          v-for="item in history"
          :key="item.id"
          class="space-y-1"
        >
          <div class="flex items-center gap-2 text-white">
            <span class="text-primary-400">$</span>
            <span class="font-semibold">{{ item.command }}</span>
          </div>

          <pre
            v-if="item.result.stdout"
            class="whitespace-pre-wrap text-gray-300"
          >{{ item.result.stdout }}</pre>
          <pre
            v-if="item.result.stderr"
            class="whitespace-pre-wrap text-red-400"
          >{{ item.result.stderr }}</pre>
          <div
            v-if="item.result.error"
            class="text-red-400"
          >
            Error: {{ item.result.error }}
          </div>
        </div>
      </div>

      <!-- Command Input -->
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
