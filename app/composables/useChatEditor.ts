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

  function handleKeydown(e: KeyboardEvent, submit: () => void) {
    if (e.isComposing || e.keyCode === 229) return

    if (e.key === 'Enter') {
      if (e.shiftKey) {
        // Default Tiptap handles Shift+Enter for newline
      } else {
        const shouldSubmit = typeof sendOnEnter === 'boolean' ? sendOnEnter : sendOnEnter.value
        if (shouldSubmit) {
          e.preventDefault()
          submit()
        }
      }
    }
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
