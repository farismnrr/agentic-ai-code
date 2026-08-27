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
  index,
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
  /** Authorization role. The default account role is intentionally least-privileged. */
  role: text('role').$type<'user' | 'admin'>().notNull().default('user'),
  /** Pending primary-email change; the new address is not authoritative until confirmed. */
  pendingEmail: text('pending_email'),
  pendingEmailTokenHash: text('pending_email_token_hash'),
  pendingEmailExpiresAt: timestamp('pending_email_expires_at', { withTimezone: true }),
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
// Browser sessions
// ---------------------------------------------------------------------------

export const authSessions = aiCode.table('auth_sessions', {
  /** Opaque identifier safe to expose only after owner scoping. */
  id: uuid('id').primaryKey(),
  userId: uuid('user_id')
    .notNull()
    .references(() => users.id, { onDelete: 'cascade' }),
  /** SHA-256 of the sealed-cookie session secret; the bearer value is never persisted. */
  secretHash: text('secret_hash').notNull().unique(),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  lastSeenAt: timestamp('last_seen_at', { withTimezone: true }).notNull().defaultNow(),
  revokedAt: timestamp('revoked_at', { withTimezone: true })
}, table => [
  index('auth_sessions_user_idx').on(table.userId, table.createdAt)
])

// ---------------------------------------------------------------------------
// TOTP MFA and recovery codes
// ---------------------------------------------------------------------------

export const mfaFactors = aiCode.table('mfa_factors', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id')
    .notNull()
    .references(() => users.id, { onDelete: 'cascade' }),
  type: text('type').$type<'totp'>().notNull(),
  /** Encrypted TOTP secret. Plaintext exists only during enrollment/verification. */
  secretEncrypted: text('secret_encrypted').notNull(),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  confirmedAt: timestamp('confirmed_at', { withTimezone: true }),
  revokedAt: timestamp('revoked_at', { withTimezone: true })
}, table => [
  index('mfa_factors_user_idx').on(table.userId, table.createdAt)
])

export const mfaRecoveryCodes = aiCode.table('mfa_recovery_codes', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id')
    .notNull()
    .references(() => users.id, { onDelete: 'cascade' }),
  /** SHA-256 of a one-time recovery code. */
  codeHash: text('code_hash').notNull().unique(),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  usedAt: timestamp('used_at', { withTimezone: true })
}, table => [
  index('mfa_recovery_codes_user_idx').on(table.userId, table.usedAt)
])

// ---------------------------------------------------------------------------
// Persistent security audit
// ---------------------------------------------------------------------------

export const securityEvents = aiCode.table('security_events', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id').references(() => users.id, { onDelete: 'set null' }),
  actorUserId: uuid('actor_user_id').references(() => users.id, { onDelete: 'set null' }),
  eventType: text('event_type').notNull(),
  outcome: text('outcome').$type<'ok' | 'denied' | 'error' | 'challenged'>().notNull(),
  /** Allowlisted bounded metadata only; never request bodies or bearer values. */
  metadata: jsonb('metadata').$type<Record<string, string | number | boolean>>().notNull().default({}),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow()
}, table => [
  index('security_events_user_created_idx').on(table.userId, table.createdAt)
])

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
  mode: text('mode').$type<'chat' | 'agent'>().notNull().default('chat'),
  permissionMode: text('permission_mode').$type<'plan' | 'bypass' | 'manual'>().notNull().default('manual'),
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

// ---------------------------------------------------------------------------
// Workspace activity ledger
// ---------------------------------------------------------------------------

export type ActivityStatus = 'started' | 'running' | 'ok' | 'error' | 'denied' | 'cancelled' | 'interrupted'

export const relayActivitySources = aiCode.table('relay_activity_sources', {
  id: uuid('id').primaryKey(),
  userId: uuid('user_id').notNull().references(() => users.id, { onDelete: 'cascade' }),
  deviceId: uuid('device_id').references(() => userDevices.id, { onDelete: 'set null' }),
  label: varchar('label', { length: 80 }).notNull(),
  kind: varchar('kind', { length: 32 }).notNull(),
  /** Stable source identifier generated by the relay; populated on first ingest. */
  sourceKey: varchar('source_key', { length: 256 }).unique(),
  tokenHash: text('token_hash').notNull().unique(),
  tokenPrefix: varchar('token_prefix', { length: 16 }).notNull(),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  lastSeenAt: timestamp('last_seen_at', { withTimezone: true }),
  revokedAt: timestamp('revoked_at', { withTimezone: true })
}, table => [index('relay_activity_sources_user_idx').on(table.userId, table.createdAt)])

export const relayActivityWorkspaceBindings = aiCode.table('relay_activity_workspace_bindings', {
  id: uuid('id').primaryKey().defaultRandom(),
  sourceId: uuid('source_id').notNull().references(() => relayActivitySources.id, { onDelete: 'cascade' }),
  workspaceId: uuid('workspace_id').notNull().references(() => workspaces.id, { onDelete: 'cascade' }),
  rootFingerprint: varchar('root_fingerprint', { length: 128 }).notNull(),
  clearThroughSequence: integer('clear_through_sequence').notNull().default(0),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  lastSeenAt: timestamp('last_seen_at', { withTimezone: true }).notNull().defaultNow()
}, table => [unique('relay_activity_binding_unique').on(table.sourceId, table.rootFingerprint), index('relay_activity_binding_workspace_idx').on(table.workspaceId)])

export const workspaceActivity = aiCode.table('workspace_activity', {
  id: uuid('id').primaryKey().defaultRandom(),
  sourceId: uuid('source_id').notNull().references(() => relayActivitySources.id, { onDelete: 'cascade' }),
  activityId: varchar('activity_id', { length: 256 }).notNull(),
  workspaceId: uuid('workspace_id').notNull().references(() => workspaces.id, { onDelete: 'cascade' }),
  sourceSequence: integer('source_sequence').notNull(),
  contractVersion: varchar('contract_version', { length: 24 }).notNull(),
  actor: varchar('actor', { length: 256 }).notNull(),
  actorSource: varchar('actor_source', { length: 256 }),
  channel: varchar('channel', { length: 256 }).notNull(),
  clientInfoName: varchar('client_info_name', { length: 256 }),
  clientInfoVersion: varchar('client_info_version', { length: 64 }),
  tool: varchar('tool', { length: 256 }).notNull(),
  category: varchar('category', { length: 40 }).notNull(),
  effects: jsonb('effects').$type<string[]>().notNull().default([]),
  status: text('status').$type<ActivityStatus>().notNull(),
  target: varchar('target', { length: 4096 }).notNull(),
  startedAt: timestamp('started_at', { withTimezone: true }).notNull(),
  finishedAt: timestamp('finished_at', { withTimezone: true }),
  durationMs: integer('duration_ms'),
  changeEvidence: jsonb('change_evidence').$type<Record<string, unknown>>(),
  occurredAt: timestamp('occurred_at', { withTimezone: true }).notNull(),
  ingestedAt: timestamp('ingested_at', { withTimezone: true }).notNull().defaultNow()
}, table => [unique('workspace_activity_source_activity_unique').on(table.sourceId, table.activityId), unique('workspace_activity_source_sequence_unique').on(table.sourceId, table.sourceSequence), index('workspace_activity_cursor_idx').on(table.workspaceId, table.startedAt, table.id), index('workspace_activity_source_sequence_idx').on(table.sourceId, table.sourceSequence)])

export const workspaceActivityPayloads = aiCode.table('workspace_activity_payloads', {
  id: uuid('id').primaryKey().defaultRandom(),
  activityId: uuid('activity_id').notNull().references(() => workspaceActivity.id, { onDelete: 'cascade' }),
  payloadKind: varchar('payload_kind', { length: 40 }).notNull(),
  payloadVersion: varchar('payload_version', { length: 24 }).notNull(),
  encryptedEnvelope: text('encrypted_envelope').notNull(),
  checksum: varchar('checksum', { length: 128 }).notNull(),
  byteCount: integer('byte_count').notNull(),
  complete: boolean('complete').notNull().default(true),
  chunkIndex: integer('chunk_index').notNull().default(0),
  chunkCount: integer('chunk_count').notNull().default(1),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow()
}, table => [unique('workspace_activity_payload_unique').on(table.activityId, table.payloadKind, table.chunkIndex)])
