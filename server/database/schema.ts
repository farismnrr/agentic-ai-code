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
  reasoningEffort: text('reasoning_effort').$type<'low' | 'medium' | 'high' | 'max'>(),
  enabledToolIds: jsonb('enabled_tool_ids').$type<string[]>().notNull().default([]),
  approvals: jsonb('approvals').$type<Record<string, 'always' | 'never'>>().notNull().default({}),
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
  defaultModelId: text('default_model_id').notNull(),
  temperature: real('temperature').notNull().default(0.7),
  systemPrompt: text('system_prompt').notNull().default(''),
  displayName: text('display_name').notNull(),
  email: text('email').notNull()
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
