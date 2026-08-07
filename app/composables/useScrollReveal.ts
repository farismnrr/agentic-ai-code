/**
 * Reveals an element the first time it scrolls into view.
 *
 * Deliberately one-shot: re-animating on every pass makes a page feel
 * twitchy when someone scrolls back up, and the reveal has already done its
 * job of drawing the eye down the page.
 *
 * Elements start hidden only once JS confirms it will run — a `v-if` on the
 * reveal state would leave content invisible if the script fails, which is a
 * worse failure than no animation.
 */
export function useScrollReveal() {
  const el = ref<HTMLElement | null>(null)
  const revealed = ref(true)

  onMounted(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return

    revealed.value = false

    const observer = new IntersectionObserver(([entry]) => {
      if (!entry?.isIntersecting) return
      revealed.value = true
      observer.disconnect()
    }, { threshold: 0.1, rootMargin: '0px 0px -8% 0px' })

    if (el.value) observer.observe(el.value)
    onBeforeUnmount(() => observer.disconnect())
  })

  return { el, revealed }
}
