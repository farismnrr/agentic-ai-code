<script setup lang="ts">
/**
 * Wraps a landing section so it rises into place once, on first view.
 * Translation is small on purpose — a large travel reads as a slideshow.
 */
const { el, revealed } = useScrollReveal()
</script>

<template>
  <div
    ref="el"
    class="transition-[opacity,transform] duration-500 ease-[cubic-bezier(0.25,1,0.5,1)]"
    :class="revealed ? 'translate-y-0 opacity-100' : 'translate-y-3 opacity-0'"
  >
    <slot />
  </div>
</template>
