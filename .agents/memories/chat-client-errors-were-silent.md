---
name: chat-client-errors-were-silent
description: useConversationChat's onError only console.error'd — a provider failure (503/401/429 from the AI SDK) looked like an unresponsive chat with zero UI feedback
metadata:
  type: feedback
---

`useChat`'s `onError` in `app/composables/useConversationChat.ts` used to only `console.error` — when the provider call fails (confirmed via Loki logs: `503 Endpoint is unavailable`, then `401 Provider returned error` on retry), the UI just sits there with no toast, no visible error, nothing but devtools console. `UChatPrompt`'s `:error="error"` prop was already wired in `chat/[id].vue`, but the AI SDK's error `message` is a raw JSON blob (e.g. `[503]: {"error":{"message":"Upstream request failed..."}}`), not something presentable on its own.

**Why this matters:** a user reported "I asked something and it just didn't respond" — root cause traced through Loki (`sensio-loki` container, on the `sensio-network` docker network, no host port published — query it via a throwaway container joined to that network, e.g. `docker run --rm --network sensio-network curlimages/curl -s -G http://<loki-ip>:3100/loki/api/v1/query_range ...`) to an upstream provider outage, not a bug in the chat flow. But nothing in the app itself signaled that anything had failed.

**Fix shipped:** `onError` now also fires a `toast.add({ color: 'error', ... })`, with a `friendlyChatErrorMessage()` helper that regexes/`JSON.parse`s the `{...}` blob out of `error.message` and surfaces `parsed.error.message` when present, falling back to the raw string otherwise.

**How to apply:** if provider errors need to be silenced or de-duplicated (e.g. don't toast on every retry within a burst), that logic belongs in this same `onError`, not scattered across pages — `useConversationChat` is the only place `useChat` is constructed (verified: `chat/[id].vue` is currently its one caller).
