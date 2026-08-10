<script setup lang="ts">
import type { RelayExecResult } from '~/composables/useRelayAgent'

useSeoMeta({ title: 'Local Terminal' })

const {
  sessionCredential,
  port,
  isConnected,
  isConnecting,
  error,
  pair,
  connect,
  disconnect,
  exec,
  setSessionCredential
} = useRelayAgent()

const pairingTokenInput = ref('')
const pairingPending = ref(false)
const toast = useToast()

const commandInput = ref('')
const execPending = ref(false)
const history = ref<Array<{ id: string, command: string, result: RelayExecResult }>>([])

onMounted(() => {
  if (sessionCredential.value && !isConnected.value) {
    void connect()
  }
})

async function handlePair() {
  if (!pairingTokenInput.value.trim()) return
  pairingPending.value = true
  const success = await pair(pairingTokenInput.value.trim())
  pairingPending.value = false

  if (success) {
    toast.add({ title: 'Paired successfully', icon: 'i-lucide-check', color: 'success' })
    pairingTokenInput.value = ''
  } else {
    toast.add({ title: 'Pairing failed', description: error.value || 'Invalid token', color: 'error' })
  }
}

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

function handleUnpair() {
  setSessionCredential(null)
  disconnect()
  history.value = []
  toast.add({ title: 'Unpaired local relay agent', color: 'neutral' })
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
        Terminal data never leaves your computer over the internet.
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
            :label="isConnected ? 'Connected' : sessionCredential ? 'Disconnected' : 'Not Paired'"
            :color="isConnected ? 'success' : sessionCredential ? 'warning' : 'neutral'"
            variant="subtle"
          />
        </div>
      </template>

      <div
        v-if="sessionCredential"
        class="space-y-4"
      >
        <div class="flex items-center justify-between text-sm">
          <span class="text-muted">Target Host</span>
          <code class="font-mono text-highlighted">http://127.0.0.1:{{ port }}</code>
        </div>

        <div class="flex gap-2">
          <UButton
            v-if="!isConnected"
            label="Reconnect"
            icon="i-lucide-refresh-cw"
            :loading="isConnecting"
            color="primary"
            @click="connect"
          />
          <UButton
            label="Unpair Agent"
            icon="i-lucide-link-2-off"
            color="error"
            variant="outline"
            @click="handleUnpair"
          />
        </div>
      </div>

      <div
        v-else
        class="space-y-4"
      >
        <p class="text-sm text-muted">
          Run <code class="rounded bg-elevated px-1 py-0.5 font-mono text-highlighted">npx @ai-code/relay-agent start</code> on your machine, then enter the pairing token printed in your terminal below:
        </p>

        <div class="flex max-w-md gap-2">
          <UInput
            v-model="pairingTokenInput"
            placeholder="Enter pairing token..."
            class="flex-1 font-mono"
            :disabled="pairingPending"
            @keyup.enter="handlePair"
          />
          <UButton
            label="Pair"
            icon="i-lucide-link"
            :loading="pairingPending"
            @click="handlePair"
          />
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
          <span class="font-medium text-highlighted">Scoped Local Terminal</span>
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
          Terminal ready. Type a command below to execute in your scoped workspace.
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
