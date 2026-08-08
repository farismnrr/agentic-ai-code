import { ref, type Ref } from 'vue'

export function useChatEditor(input: Ref<string>, sendOnEnter: Ref<boolean> | boolean) {
  const editorRef = ref()

  function syncText() {
    if (editorRef.value?.editor) {
      input.value = editorRef.value.editor.getText()
    }
  }

  function clearEditor() {
    if (editorRef.value?.editor) {
      editorRef.value.editor.commands.clearContent()
    }
    input.value = ''
  }

  // Returns true when the keystroke was handled (submitted), so the caller
  // (Tiptap's own editorProps.handleKeyDown — see usage below) knows to
  // stop further processing. A plain `@keydown` listener on <UEditor> does
  // NOT work: Nuxt UI's Editor.vue only declares `update:modelValue` as a
  // real emit, so any other listener attr (like `onKeydown`) falls through
  // to ProseMirror's `editorProps.attributes`, which stringifies it into an
  // inert `onkeydown="..."` HTML attribute instead of registering a real
  // event listener. Tiptap's `editorProps.handleKeyDown(view, event)` is
  // the actual, supported hook for intercepting keystrokes.
  function handleKeydown(e: KeyboardEvent, submit: () => void): boolean {
    if (e.isComposing || e.keyCode === 229) return false

    if (e.key === 'Enter' && !e.shiftKey) {
      const shouldSubmit = typeof sendOnEnter === 'boolean' ? sendOnEnter : sendOnEnter.value
      if (shouldSubmit) {
        e.preventDefault()
        submit()
        return true
      }
    }
    return false
  }

  const mentionItems = [
    { label: 'search', description: 'Force this turn to search the web', icon: 'i-lucide-search' }
  ]

  return {
    editorRef,
    syncText,
    clearEditor,
    handleKeydown,
    mentionItems
  }
}
