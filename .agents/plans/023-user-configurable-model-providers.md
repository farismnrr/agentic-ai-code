# Plan: User-configurable model providers (9Router + GCP Agent Platform)

## Context

Chat models are currently hardcoded in `shared/utils/models.ts` (3 entries), and the
only provider is 9Router, wired via a single env-var API key/base URL
(`server/utils/router-model.ts`, `server/utils/langgraph-model.ts`). There's no way
for a user to add their own models, switch providers, or tune per-model behavior
(context window, max output tokens, thinking/reasoning budget).

This plan makes providers and models user-owned, DB-backed resources: a user pastes
an API key for **9Router** or **GCP Agent Platform**, then freely adds/edits/removes
models under that provider, with global defaults for context window / max output /
thinking that any model can override.

## Data model

Two new tables in `server/database/schema.ts`, following the existing `mcpServers`
per-user-ownership pattern:

```ts
export const modelProviders = aiCode.table('model_providers', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id').notNull().references(() => users.id, { onDelete: 'cascade' }),
  type: text('type').$type<'9router' | 'gcp_agent_platform'>().notNull(),
  name: text('name').notNull(), // user-facing label, e.g. "My 9Router"
  baseUrl: text('base_url'), // 9router only; null for gcp
  apiKeyEncrypted: text('api_key_encrypted').notNull(), // AES-256-GCM, iv+tag+ciphertext
  enabled: boolean('enabled').notNull().default(true),
  createdAt: timestamp(...).notNull().defaultNow(),
  updatedAt: timestamp(...).notNull().defaultNow()
})

export const models = aiCode.table('models', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id').notNull().references(() => users.id, { onDelete: 'cascade' }),
  providerId: uuid('provider_id').notNull().references(() => modelProviders.id, { onDelete: 'cascade' }),
  modelId: text('model_id').notNull(), // wire name sent to the provider, e.g. "vx/gemini-3-flash-preview"
  label: text('label').notNull(),
  description: text('description').notNull().default(''),
  icon: text('icon').notNull().default('i-lucide-sparkles'),
  contextWindow: integer('context_window'), // null = use global default
  maxOutputTokens: integer('max_output_tokens'), // null = use global default
  thinkingEnabled: boolean('thinking_enabled'), // null = use global default
  thinkingMinTokens: integer('thinking_min_tokens'), // null = use global default
  thinkingMaxTokens: integer('thinking_max_tokens'), // null = use global default
  createdAt: timestamp(...).notNull().defaultNow(),
  updatedAt: timestamp(...).notNull().defaultNow()
})
```

`userSettings` gains global defaults (nullable overrides fall back to these) plus
swaps `defaultModelId` (currently a free string) to reference `models.id`:

```ts
defaultModelId: uuid('default_model_id').references(() => models.id, { onDelete: 'set null' }),
defaultContextWindow: integer('default_context_window').notNull().default(128000),
defaultMaxOutputTokens: integer('default_max_output_tokens').notNull().default(8192),
defaultThinkingEnabled: boolean('default_thinking_enabled').notNull().default(false),
defaultThinkingMinTokens: integer('default_thinking_min_tokens').notNull().default(1024),
defaultThinkingMaxTokens: integer('default_thinking_max_tokens').notNull().default(8192)
```

`conversations.modelId` stays `text` (no FK — conversations must keep working even
if a model is later deleted) but now stores a `models.id` UUID string instead of the
old hardcoded id.

## Secret encryption

No encryption utility exists yet (env-only secrets today). Add
`server/utils/crypto.ts` with `encryptSecret`/`decryptSecret` using Node's built-in
`node:crypto` AES-256-GCM, keyed off a new runtime config value
`modelProviderSecretKey` (env `NUXT_MODEL_PROVIDER_SECRET_KEY`, 32-byte hex, added to
`nuxt.config.ts` next to `routerApiKey`). Same file style as `server/utils/api-key.ts`
(random iv per encrypt call, store `iv:tag:ciphertext` hex-joined).

## Provider abstraction (replaces hardcoded 9Router-only path)

New `server/utils/providers/` folder:
- `router9.ts` — moves today's `createOpenAICompatible` logic from `router-model.ts`, now taking `{ baseUrl, apiKey }` from a decrypted `modelProviders` row instead of runtime config.
- `gcp-agent-platform.ts` — new, uses `@ai-sdk/google` (Gemini API-key auth path, not `@ai-sdk/google-vertex` since no project/location/service-account is being collected) keyed off the decrypted API key.
- `index.ts` — `getChatModel(provider: ModelProviderRow, modelId: string)` dispatches on `provider.type` and returns an AI SDK `LanguageModel`, replacing `getRouterModel()`. Update `server/api/chat.post.ts` (and wherever `getRouterModel`/`getLanggraphModel` are called) to first load the model's row (joins `models` → `modelProviders`), decrypt the key, and call this dispatcher.
- `langgraph-model.ts` equivalent: same dispatch, `ChatOpenAI` for 9router, `ChatGoogleGenerativeAI` (`@langchain/google-genai`) for GCP.

Effective config resolution (context window / max output / thinking) becomes a small
helper `resolveModelConfig(model, settings)` that returns model-level values falling
back to `userSettings` defaults — used wherever these values feed the AI SDK call
(`streamText`/`ChatOpenAI` options) and the LangGraph path.

## Server API

New CRUD routes mirroring `server/api/mcp-servers/*`:
- `server/api/providers/index.get.ts`, `index.post.ts`, `[id].put.ts`, `[id].delete.ts` — provider CRUD. POST/PUT encrypt the incoming raw API key before storing; GET never returns the decrypted key or ciphertext (return `hasApiKey: boolean` instead), matching how `apiKeys` never re-exposes `keyHash`.
- `server/api/models/index.get.ts`, `index.post.ts`, `[id].put.ts`, `[id].delete.ts` — model CRUD, scoped to a `providerId` owned by the requesting user (ownership check like `server/utils/mcp-servers.ts` does for `mcpServers`).
- `server/utils/providers.ts` and `server/utils/models.ts` — the query/mutation logic backing the above, same shape as `server/utils/mcp-servers.ts`.
- `server/api/settings.put.ts` / `settings.ts` — extend the updatable fields with the new default* columns and change `defaultModelId` handling to a UUID FK.

## Shared types & frontend

- `shared/types/chat.ts`: replace the static `ChatModel` shape's assumptions — it becomes the DB row shape (`id`, `providerId`, `modelId`, `label`, `description`, `icon`, plus the optional override fields). Add `ModelProvider` type (`id`, `type`, `name`, `baseUrl`, `enabled`, `hasApiKey` — no secret fields client-side).
- Delete `shared/utils/models.ts` (hardcoded list) once nothing imports it — `defaultModelId` constant goes away; "no default set yet" becomes a real empty/loading state.
- `app/composables/useSettings.ts`: fetch defaults from `/api/settings` as today; add composable(s) `useModelProviders()` / `useModels()` (fetch/create/update/delete), same pattern as existing `useSettings()`.
- `app/pages/settings/models.vue`: rework into two sections — "Providers" (list of `modelProviders` with add/edit/remove, type picker, API key input write-only) and "Models" (list of `models` grouped by provider, add/edit/remove, per-model override fields for context window / max output / thinking enable + min-max, each defaulting to "use global" until touched) — plus the existing global default-model/temperature/system-prompt controls now sourced from `userSettings`' new default* fields.
- Anywhere `models` from `shared/utils/models.ts` was imported for lookup (chat UI model picker, reasoning-effort gating via `supportsReasoning`) switches to the fetched `useModels()` list; `supportsReasoning` becomes `thinkingEnabled` per model.

## Migration & seeding

- Drizzle migration adds the two new tables and the `userSettings`/`conversations` column changes.
- One-off seed step (script under `scripts/`, same style as `scripts/backfill-models.ts`) run once per existing user: create a `model_providers` row of type `9router` from current `NUXT_ROUTER_BASE_URL`/`NUXT_ROUTER_API_KEY` (encrypted), insert the 3 existing hardcoded models as `models` rows under it, then remap `userSettings.defaultModelId` and every `conversations.modelId` from the old string ids to the new row ids.

## Verification

- `pnpm typecheck` / `pnpm lint` after schema + code changes.
- `pnpm --filter ai-code db:generate` (or repo's Drizzle generate command) to produce the migration, then apply it against the dev DB.
- Run the seed script against dev DB, confirm existing conversations still resolve a valid model.
- `pnpm dev`, then in-browser: Settings → Providers — add a 9Router provider and a GCP Agent Platform provider with a test key; Settings → Models — add a model under each, set a per-model thinking override; start a new chat, confirm the model picker shows the new models and a message round-trips through each provider; edit a provider's key and confirm old conversations still load (deleted-model conversations keep their stored id/label gracefully).
