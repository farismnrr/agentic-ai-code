# Loaded State Pattern for `useState`-Backed Composables

## Context

When using `useState` in Nuxt to store data fetched from the server (e.g., in composables like `useWorkspaces`, `useConversations`, `useMcpServers`), there's an ambiguity in the initial state: an empty array `[]` could mean the data is still loading, or it could mean the data has loaded and is genuinely empty.

## The Pattern

To resolve this ambiguity, especially when rendering "empty states" vs "loading skeletons" in the UI (like replacing the chat prompt with a workspace picker only if genuinely zero workspaces exist), add a `loaded` ref to the composable:

```typescript
const loaded = ref(false)

async function loadAll() {
  try {
    const data = await $fetch('/api/something')
    items.value = data
  } finally {
    loaded.value = true
  }
}

return { items, loaded, loadAll }
```

This ensures the UI can explicitly check `!loaded` to show placeholders, preventing layout shifts or flashes of empty states before the first fetch completes. This pattern was introduced in `useWorkspaces` (plan `009-workspace-picker.md`) and is worth keeping in mind for other composables like `useConversations` and `useMcpServers`.
