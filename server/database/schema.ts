import {
  pgSchema,
  uuid,
  text,
  varchar,
  timestamp,
  unique
} from 'drizzle-orm/pg-core'

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
