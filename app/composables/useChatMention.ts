import { ref, computed, watch, type Ref } from 'vue'

export function useChatMention(input: Ref<string>) {
  const forceCloseMention = ref(false)

  watch(input, () => {
    forceCloseMention.value = false
  })

  // Regex searches for whitespaces and word-chars, properly escaped
  const mentionMatch = computed(() =>
    forceCloseMention.value ? null : input.value.match(/(?:^|\s)@(\w*)$/)
  )
  const mentionOpen = computed(() => mentionMatch.value !== null)
  const mentionFilter = computed(() => (mentionMatch.value ? mentionMatch.value[1]! : ''))

  function onMentionSelect(trigger: string) {
    if (!mentionMatch.value) return
    const matchStr = mentionMatch.value[0]
    // Regex replaces trailing @word
    const replaceStr = matchStr.replace(/@\w*$/, `@${trigger} `)
    input.value = input.value.substring(0, input.value.length - matchStr.length) + replaceStr
    forceCloseMention.value = true

    // Focus textarea after selection
    setTimeout(() => {
      const textarea = document.querySelector('textarea')
      if (textarea) textarea.focus()
    }, 0)
  }

  return {
    mentionOpen,
    mentionFilter,
    onMentionSelect,
    forceCloseMention
  }
}
