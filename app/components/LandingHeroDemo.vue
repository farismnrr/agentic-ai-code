<script setup lang="ts">
import { pickScenario } from '#shared/utils/fixtures/replies'

/**
 * A conversation that runs itself: types a prompt, streams a reply, calls a
 * tool, then starts over.
 *
 * It is driven by a hardcoded set of data to ensure consistent presentation
 * on the landing page regardless of external state.
 *
 * Motion budget: this is the page's one orchestrated moment. Everything else
 * on the landing page is a scroll reveal.
 */
const PROMPT = 'Search the repo for the chat components'
const TYPE_MS = 45
const HOLD_MS = 2600

const typed = ref('')
const reply = ref('')
const toolName = ref<string | null>(null)
const toolDone = ref(false)
const phase = ref<'typing' | 'thinking' | 'streaming' | 'done'>('typing')

const root = ref<HTMLElement | null>(null)
const visible = ref(false)
let cancelled = false

/**
 * Static final frame instead of the loop. Someone who has asked for less
 * motion should still see what the product does — a blank box would punish
 * them for the preference.
 */
const reducedMotion = ref(false)

function sleep(ms: number) {
  return new Promise<void>(resolve => setTimeout(resolve, ms))
}

/** Waits while the section is off-screen, so nothing runs behind the fold. */
async function untilVisible() {
  while (!visible.value && !cancelled) await sleep(200)
}

function showFinalFrame() {
  typed.value = PROMPT
  reply.value = 'I\'ll use `search_repositories` from the github server for this.'
  toolName.value = 'github · search_repositories'
  toolDone.value = true
  phase.value = 'done'
}

async function runOnce() {
  typed.value = ''
  reply.value = ''
  toolName.value = null
  toolDone.value = false
  phase.value = 'typing'

  for (const char of PROMPT) {
    if (cancelled) return
    await untilVisible()
    typed.value += char
    await sleep(TYPE_MS)
  }

  phase.value = 'thinking'
  await sleep(500)
  if (cancelled) return

  phase.value = 'streaming'

  const chunks = pickScenario('search').build({ enabledToolIds: ['github'] })

  for (const chunk of chunks) {
    if (cancelled) break

    if (chunk.type === 'text-delta') reply.value += chunk.delta
    else if (chunk.type === 'tool-input-available') toolName.value = `github · ${chunk.toolName}`
    else if (chunk.type === 'tool-output-available') toolDone.value = true

    await sleep(26)
  }

  phase.value = 'done'
}

onMounted(async () => {
  reducedMotion.value = window.matchMedia('(prefers-reduced-motion: reduce)').matches
  if (reducedMotion.value) {
    showFinalFrame()
    return
  }

  const observer = new IntersectionObserver(
    ([entry]) => { visible.value = entry?.isIntersecting ?? false },
    { threshold: 0.15 }
  )
  if (root.value) observer.observe(root.value)

  // A backgrounded tab should not keep the loop turning.
  const onVisibility = () => {
    if (document.hidden) visible.value = false
  }
  document.addEventListener('visibilitychange', onVisibility)

  onBeforeUnmount(() => {
    cancelled = true
    observer.disconnect()
    document.removeEventListener('visibilitychange', onVisibility)
  })

  while (!cancelled) {
    await untilVisible()
    if (cancelled) break
    await runOnce()
    await sleep(HOLD_MS)
  }
})
</script>

<template>
  <div
    ref="root"
    class="mx-auto w-full max-w-2xl overflow-hidden rounded-lg border border-default bg-elevated text-left shadow-xl"
    aria-hidden="true"
  >
    <!-- Chrome. Mono, because this is a readout, not prose. -->
    <div class="flex items-center gap-2 border-b border-default px-3 py-2">
      <span class="size-2 rounded-full bg-accented" />
      <span class="size-2 rounded-full bg-accented" />
      <span class="size-2 rounded-full bg-accented" />
      <span class="ms-2 font-mono text-xs text-dimmed">ai-code</span>
      <span
        v-if="phase === 'streaming'"
        class="ms-auto flex items-center gap-1.5 font-mono text-[11px] text-primary"
      >
        <span class="size-1.5 animate-pulse rounded-full bg-primary" />
        streaming
      </span>
    </div>

    <div class="space-y-4 p-4 text-sm sm:p-5">
      <!-- Prompt -->
      <div class="flex justify-end">
        <p class="max-w-[85%] rounded-md bg-accented px-3 py-2 text-default">
          {{ typed }}<span
            v-if="phase === 'typing'"
            class="ms-px inline-block h-4 w-px animate-pulse bg-primary align-middle"
          />
        </p>
      </div>

      <!-- Tool readout -->
      <div
        v-if="toolName"
        class="flex items-center gap-2 rounded-md border border-default bg-default px-3 py-2 font-mono text-xs"
      >
        <UIcon
          :name="toolDone ? 'i-lucide-check' : 'i-lucide-loader'"
          :class="toolDone ? 'text-primary' : 'animate-spin text-dimmed'"
        />
        <span class="text-muted">{{ toolName }}</span>
        <span
          v-if="toolDone"
          class="ms-auto text-dimmed"
        >3 results</span>
      </div>

      <!-- Reply -->
      <p
        v-if="reply || phase === 'thinking'"
        class="min-h-5 text-toned"
      >
        <span
          v-if="phase === 'thinking'"
          class="font-mono text-xs text-dimmed"
        >thinking…</span>
        <template v-else>
          {{ reply }}<span
            v-if="phase === 'streaming'"
            class="ms-px inline-block h-4 w-px animate-pulse bg-primary align-middle"
          />
        </template>
      </p>
    </div>
  </div>
</template>
