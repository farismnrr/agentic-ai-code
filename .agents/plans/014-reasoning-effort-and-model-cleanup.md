# 014 — Reasoning effort levels + kill the stray `legacy-model-id` default

## Why

Two real gaps found while manually verifying plan 013 with Playwright:

1. **A fresh conversation's model picker shows "legacy-model-id"** — not a
   9Router model at all, and not one of the three entries in
   `shared/utils/models.ts`. Traced to `server/utils/settings.ts:32`:
   `getSettings()` inserts a brand-new `userSettings` row with a
   **hardcoded `defaultModelId: 'legacy-model-id'`**, out of sync with
   `shared/utils/models.ts`'s own `export const defaultModelId =
   'vx/gemini-3-flash-preview'`. Every new user/first load gets seeded
   with a model id that matches nothing in `models`, so the picker's
   `.find()` comes back `undefined` and falls back to showing the raw
   stored string.
2. **No user control over how hard "High Thinking" thinks.** Plan 013
   wired real reasoning via `extractReasoningMiddleware`, but reasoning
   depth is fixed. The user wants a visible **effort** control — low /
   medium / high / max — the same shape as OpenAI's `reasoning_effort`
   knob, mapped onto how much the model reasons before answering.

## Phase 1 — Fix the stray default model

1. `server/utils/settings.ts:32` — replace the hardcoded
   `defaultModelId: 'legacy-model-id'` literal with
   `defaultModelId` imported from `shared/utils/models.ts`, so server and
   client agree on one source of truth.
2. **Data cleanup, not just code**: any `userSettings` row already
   persisted with `'legacy-model-id'` (and any `conversations.modelId` seeded
   from it) needs a one-off migration/backfill to
   `vx/gemini-3-flash-preview` — fixing only the code leaves existing rows
   broken. Confirm scope by querying how many rows are actually affected
   before deciding between a migration script and a lazy fix-on-read.
3. Verify: delete `userSettings` for a test user (or use the disposable QA
   account from plan 013's manual testing), reload `/chat`, confirm the
   picker now shows "Flash Preview" — never a raw id string — for a truly
   fresh user.

## Phase 2 — Reasoning effort control

**Mechanism, confirmed by reading the installed SDK
(`@ai-sdk/openai-compatible@3.0.25`):** `providerOptions['9router'].reasoningEffort`
(string) is read by the provider's `getArgs()` and forwarded verbatim as
`reasoning_effort` in the outgoing HTTP body to 9Router — this is
OpenAI's own real API field name, not something invented. `ai@7`'s
`streamText` has no *generic* effort option; it only exists at the
provider-options layer, keyed by the provider's registered name (`'9router'`,
set in `server/utils/router-model.ts`) or the generic `openaiCompatible` key.

**Open risk to resolve before committing to four levels:** OpenAI's real
`reasoning_effort` only defines `low`/`medium`/`high` (no `max`). Whether
9Router's downstream model accepts an unrecognized `'max'` value
gracefully (ignores it / clamps to `high`) or errors is unknown — the SDK
schema is just `z.string()`, so nothing client-side stops us from sending
it, but the receiving end might reject it. **Resolve this with a live call
first** (same throwaway-script approach as plan 013 phase 1): hit
`high-thinking-models` via 9Router with `reasoning_effort: 'max'` and see
what comes back. If it 400s or is silently ignored, either drop to three
levels (`low`/`medium`/`high`) or confirm with whoever operates 9Router
whether `'max'` is a supported alias on their side.

1. Add `reasoningEffort?: 'low' | 'medium' | 'high' | 'max'` to the
   `Conversation` type (`shared/types/chat.ts`) alongside `modelId` — it's
   per-conversation state, same lifecycle as the model picker.
2. In `server/api/chat.post.ts`, when the resolved model has
   `supportsReasoning`, pass
   `providerOptions: { '9router': { reasoningEffort: conv.reasoningEffort ?? 'medium' } }`
   into `streamText`. Omit `providerOptions` entirely for non-reasoning
   models — no reason to send a field the provider will ignore anyway.
3. UI: a small effort selector next to the model picker in
   `app/pages/chat/[id].vue`'s `UChatPrompt` footer (same slot as
   `USelect` for `modelId`), **visible only when the selected model's
   `supportsReasoning` is true** — showing it for Flash/Free models would
   imply a control that does nothing.
4. Persist the choice the same way `modelId` already is — via
   `update(conversation.id, { reasoningEffort: value })`.
5. Manually verify with the real "High Thinking" model at each of the
   confirmed-working levels: reasoning block still renders, and (as far as
   observable from the outside) higher effort visibly reasons longer/more
   thoroughly than `low`. This is a real live check, not just "the request
   didn't error."

## Out of scope

- No retroactive reasoning-effort default per-model beyond `'medium'`
  (matches the SDK's own documented default).
- No effort control surfaced for non-reasoning models — see step 3.
- Not re-opening plan 013's animation work — this plan is additive to it.

## On completion

- [x] Phase 1 — `server/utils/settings.ts` now imports `defaultModelId`
      from `shared/utils/models.ts` instead of the hardcoded
      `'legacy-model-id'` literal.
- [x] Phase 1 — data cleanup done via `scripts/backfill-models.ts`, a
      one-off script backfilling any `user_settings.default_model_id` /
      `conversations.model_id` row still stuck on `'legacy-model-id'` to
      `vx/gemini-3-flash-preview`. First cut of the script imported
      `dotenv/config` without `dotenv` being a declared dependency
      (only present transitively in the lockfile) — it threw
      `ERR_MODULE_NOT_FOUND` before ever touching the database, caught in
      review by actually running it, not just reading the diff. Fixed by
      adding `dotenv` to `package.json` `dependencies`. Live-verified
      after the fix: `npx tsx scripts/backfill-models.ts` runs clean
      end-to-end against the real dev database (0 stale rows at the time,
      confirming both the query and the connection path work).
- [x] Phase 2 — real reasoning-effort control wired via
      `providerOptions['9router'].reasoningEffort`, confirmed correct
      against the installed `@ai-sdk/openai-compatible` source (forwarded
      verbatim as `reasoning_effort` in the outgoing HTTP body).
- [x] Phase 2's flagged open risk — whether 9Router accepts the
      non-standard `'max'` level — **resolved live, not just theorized**:
      sent a real message through "High Thinking" at `reasoningEffort:
      'max'` via the running app and confirmed both no error response and
      a visibly deeper reasoning trace (a full step-by-step long
      multiplication) versus the near-instant one-line reasoning at
      `'medium'` on the same model. `'max'` is accepted as-is.
- [x] Effort selector added to **both** places a conversation's model can
      be picked — `app/pages/chat/[id].vue` (existing conversation) and
      `app/pages/chat/index.vue` (the "New chat" landing prompt, which has
      its own separate `UChatPrompt`/model picker). The first review pass
      only found it wired into `[id].vue`; a conversation started from the
      landing page with "High Thinking" already selected had no way to
      set effort before the first message. Fixed by threading
      `reasoningEffort` through `useConversations().create()` →
      `POST /api/conversations` (validated with
      `v.picklist(['low','medium','high','max'])`) → the new column.
- [x] `pnpm build && vue-tsc -p .nuxt/tsconfig.json --noEmit && pnpm run
      lint && pnpm audit` all clean.
- [x] Merged to `dev` via PR (see plans/README.md), branch and worktree
      cleaned up per `.agents/knowledge/git.md`.
