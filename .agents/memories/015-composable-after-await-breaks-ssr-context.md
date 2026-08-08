---
name: 015-composable-after-await-breaks-ssr-context
description: calling a Nuxt composable (useState, useCookie, useSettings, etc.) after an await inside a plain async function silently breaks SSR — surfaces only as NUXT_E1001, swallowed by Promise.allSettled with no other symptom
metadata:
  type: feedback
---

Calling any Nuxt composable that needs the Nuxt/Vue app instance (`useState`, `useCookie`, and anything built on them like `useSettings()`) **after an `await` boundary inside a plain composable function** (not a component's `<script setup>`, which Nuxt's compiler specially transforms to preserve context across top-level awaits) loses the SSR request's async context. It does not throw where you'd expect — it throws `NUXT_E1001` deep inside `useNuxtApp()`, and if that call happens inside a promise that's part of a `Promise.allSettled(...)`, the rejection is silently absorbed with **zero visible symptom**: no error in the response, no crash, just whatever depended on that call's result silently never happening.

**Why this matters:** hit for real in plan [[015-persist-active-workspace]]. `useWorkspaces.ts`'s `loadAll()` called `useSettings()` *after* its own internal `await Promise.all([...])` — this broke on every request, but was invisible for a long time because the failure was wrapped in the layout's outer `Promise.allSettled`. The only way it surfaced was writing debug output to a file with `fs.appendFileSync` (bypassing Node's stdout buffering, which was *also* masking things — see [[background-command-output]]) and inspecting the rejection reason's stack trace directly; `console.error` inside the failing path never even ran, since the composable call itself is what threw.

A related, distinct trap in the same investigation: restructuring an *already-working* parallel `Promise.allSettled([...])` into a sequential `await x(); await Promise.allSettled([...])` — even with a `try/catch` around the first await — was enough on its own to reproduce the same `NUXT_E1001`, before any composable-after-await mistake was even introduced inside the called functions. The mere act of inserting an extra await/tick ahead of the batch was the trigger.

**How to apply:**
- Inside any composable (not a component setup function), call every other composable you need **synchronously, before any `await`** — grab the refs first, read `.value` (a plain property access, never itself a composable call) after the await.
- Never restructure a working `Promise.allSettled([a, b, c])` into a sequential `await a(); await Promise.allSettled([b, c])` without testing SSR specifically — it can silently break Nuxt's context propagation for `b`/`c` even if `a` itself is fine.
- If something inside a `Promise.allSettled` isn't taking effect and there's no error anywhere, don't trust `console.error`/`console.log` from a background `pnpm preview` process — Node buffers stdout to a non-TTY file (see [[background-command-output]]). Use `fs.appendFileSync` for a debug probe, or kill -TERM the process to force a flush, and log the actual `Promise.allSettled` results (`.map(r => r.status === 'rejected' ? String(r.reason) : 'ok')`), not just assume the promises resolved.
