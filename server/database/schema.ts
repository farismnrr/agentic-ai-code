import {
  pgSchema,
  uuid,
  text,
  varchar,
  timestamp,
  unique,
  jsonb,
  boolean,
  real,
  integer,
  type AnyPgColumn
} from 'drizzle-orm/pg-core'
import type { McpTool, UIMessage } from '#shared/types/chat'

/**
 * All tables live in the `ai_code` schema inside the shared `masihawam`
 * database. This follows the pattern used by other projects on this machine
 * (sensio-iot, sensio-notes, tuya_manager) — separate schemas, not separate
 * databases, so a single container serves everything.
 *
 * The connection URL adds `?search_path=ai_code` so Drizzle resolves
 * unqualified names against this schema.
 */
export const aiCode = pgSchema('ai_code')

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

export const users = aiCode.table('users', {
  id: uuid('id').primaryKey().defaultRandom(),
  email: text('email').notNull().unique(),
  name: text('name').notNull(),
  /** Nullable — OAuth-only accounts never set a password. */
  passwordHash: text('password_hash'),
  avatarUrl: text('avatar_url'),
  /** Set when the user clicks the verification link; null = unverified. */
  emailVerifiedAt: timestamp('email_verified_at', { withTimezone: true }),
  /** Incremented after credential-sensitive events to invalidate sealed sessions. */
  authVersion: integer('auth_version').notNull().default(0),
  lastActiveWorkspaceId: uuid('last_active_workspace_id').references((): AnyPgColumn => workspaces.id, { onDelete: 'set null' }),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow()
})

// ---------------------------------------------------------------------------
// OAuth accounts
// ---------------------------------------------------------------------------

export const oauthAccounts = aiCode.table('oauth_accounts', {
  provider: varchar('provider', { length: 32 }).notNull(),
  providerAccountId: text('provider_account_id').notNull(),
  userId: uuid('user_id')
    .notNull()
    .references(() => users.id, { onDelete: 'cascade' }),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow()
}, table => [
  unique().on(table.provider, table.providerAccountId)
])

// ---------------------------------------------------------------------------
// Verification tokens (email verification + password reset)
// ---------------------------------------------------------------------------

export type TokenType = 'email_verify' | 'password_reset'

export const verificationTokens = aiCode.table('verification_tokens', {
  /** SHA-256 hex of the raw token sent to the user's inbox. */
  tokenHash: text('token_hash').primaryKey(),
  userId: uuid('user_id')
    .notNull()
    .references(() => users.id, { onDelete: 'cascade' }),
  type: text('type').$type<TokenType>().notNull(),
  expiresAt: timestamp('expires_at', { withTimezone: true }).notNull(),
  /** Set once the link is clicked; prevents re-use. */
  consumedAt: timestamp('consumed_at', { withTimezone: true })
})

// ---------------------------------------------------------------------------
// Workspaces
// ---------------------------------------------------------------------------

export const workspaces = aiCode.table('workspaces', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id')
    .notNull()
    .references(() => users.id, { onDelete: 'cascade' }),
  name: text('name').notNull(),
  path: text('path').notNull(),
  pathConfirmed: boolean('path_confirmed').notNull().default(true),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow()
})

// ---------------------------------------------------------------------------
// Conversations
// ---------------------------------------------------------------------------

export const conversations = aiCode.table('conversations', {
  id: uuid('id').primaryKey().defaultRandom(),
  workspaceId: uuid('workspace_id')
    .notNull()
    .references(() => workspaces.id, { onDelete: 'cascade' }),
  userId: uuid('user_id')
    .notNull()
    .references(() => users.id, { onDelete: 'cascade' }),
  title: text('title').notNull(),
  modelId: text('model_id').notNull(),
  mode: text('mode').$type<'chat' | 'agent'>().notNull().default('agent'),
  permissionMode: text('permission_mode').$type<'plan' | 'workspace' | 'autonomous' | 'manual'>().notNull().default('manual'),
  reasoningEffort: text('reasoning_effort').$type<'low' | 'medium' | 'high' | 'max'>(),
  enabledToolIds: jsonb('enabled_tool_ids').$type<string[]>().notNull().default([]),
  approvals: jsonb('approvals').$type<Record<string, 'always' | 'never'>>().notNull().default({}),
  contextSummary: text('context_summary'),
  contextSummaryUpToMessageId: uuid('context_summary_up_to_message_id').references((): AnyPgColumn => messages.id, { onDelete: 'set null' }),
  contextSummaryUpToCreatedAt: timestamp('context_summary_up_to_created_at', { withTimezone: true }),
  lastMeasuredTokens: integer('last_measured_tokens'),
  lastMeasuredMessageId: uuid('last_measured_message_id').references((): AnyPgColumn => messages.id, { onDelete: 'set null' }),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow()
})

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

export const messages = aiCode.table('messages', {
  id: uuid('id').primaryKey().defaultRandom(),
  conversationId: uuid('conversation_id')
    .notNull()
    .references(() => conversations.id, { onDelete: 'cascade' }),
  role: text('role').notNull(),
  parts: jsonb('parts').$type<UIMessage['parts']>().notNull().default([]),
  totalTokens: integer('total_tokens'),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow()
})

// ---------------------------------------------------------------------------
// API Keys
// ---------------------------------------------------------------------------

export const apiKeys = aiCode.table('api_keys', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id')
    .notNull()
    .references(() => users.id, { onDelete: 'cascade' }),
  name: text('name').notNull(),
  keyHash: text('key_hash').notNull().unique(),
  keyPrefix: text('key_prefix').notNull(),
  lastUsedAt: timestamp('last_used_at', { withTimezone: true }),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow()
})

// ---------------------------------------------------------------------------
// User Settings
// ---------------------------------------------------------------------------

export const userSettings = aiCode.table('user_settings', {
  userId: uuid('user_id').primaryKey().references(() => users.id, { onDelete: 'cascade' }),
  language: text('language').notNull().default('en'),
  streaming: boolean('streaming').notNull().default(true),
  sendOnEnter: boolean('send_on_enter').notNull().default(true),
  defaultModelId: uuid('default_model_id').references(() => models.id, { onDelete: 'set null' }),
  temperature: real('temperature').notNull().default(0.7),
  systemPrompt: text('system_prompt').notNull().default(''),
  displayName: text('display_name').notNull(),
  email: text('email').notNull()
})

// ---------------------------------------------------------------------------
// Model Providers
// ---------------------------------------------------------------------------

export type ModelProviderType = 'openai_compatible' | 'anthropic_compatible' | 'vertex_ai'

export const modelProviders = aiCode.table('model_providers', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id').notNull().references(() => users.id, { onDelete: 'cascade' }),
  type: text('type').$type<ModelProviderType>().notNull(),
  name: text('name').notNull(),
  baseUrl: text('base_url'),
  apiKeyEncrypted: text('api_key_encrypted').notNull(),
  // Values are ciphertext (`encryptSecret`/`decryptSecret` in
  // `server/infrastructure/security/crypto.ts`), same as `apiKeyEncrypted` — custom gateway
  // headers routinely carry credentials (`Authorization`, `X-Api-Key`).
  // Keys (header names) stay plaintext; only values are secret.
  customHeaders: jsonb('custom_headers').$type<Record<string, string>>().notNull().default({}),
  enabled: boolean('enabled').notNull().default(true),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow()
})

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

export const models = aiCode.table('models', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id').notNull().references(() => users.id, { onDelete: 'cascade' }),
  providerId: uuid('provider_id').notNull().references(() => modelProviders.id, { onDelete: 'cascade' }),
  modelId: text('model_id').notNull(),
  label: text('label').notNull(),
  description: text('description').notNull().default(''),
  icon: text('icon').notNull().default('i-lucide-sparkles'),
  contextWindow: integer('context_window'),
  maxOutputTokens: integer('max_output_tokens'),
  thinkingEnabled: boolean('thinking_enabled'),
  thinkingMinTokens: integer('thinking_min_tokens'),
  thinkingMaxTokens: integer('thinking_max_tokens'),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow()
})

// ---------------------------------------------------------------------------
// MCP Servers
// ---------------------------------------------------------------------------

export const mcpServers = aiCode.table('mcp_servers', {
  id: text('id').primaryKey(),
  userId: uuid('user_id')
    .notNull()
    .references(() => users.id, { onDelete: 'cascade' }),
  name: text('name').notNull(),
  description: text('description').notNull().default(''),
  transport: text('transport').notNull(),
  url: text('url'),
  command: text('command'),
  status: text('status').notNull().default('disconnected'),
  enabled: boolean('enabled').notNull().default(true),
  tools: jsonb('tools').$type<McpTool[]>().notNull().default([]),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow()
})

// ---------------------------------------------------------------------------
// User Devices (Relay Agent Metadata)
// ---------------------------------------------------------------------------

export const userDevices = aiCode.table('user_devices', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id')
    .notNull()
    .references(() => users.id, { onDelete: 'cascade' }),
  name: text('name').notNull(),
  fingerprint: text('fingerprint').notNull(),
  pairedAt: timestamp('paired_at', { withTimezone: true }).notNull().defaultNow(),
  lastSeenAt: timestamp('last_seen_at', { withTimezone: true }).notNull().defaultNow(),
  revokedAt: timestamp('revoked_at', { withTimezone: true })
})
