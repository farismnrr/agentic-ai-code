import { useDb } from './connection'
import { and, eq, gt, isNull, sql } from 'drizzle-orm'
import { users, verificationTokens, workspaces } from '../../database/schema'

export async function findLoginUser(email: string) {
  const [user] = await useDb().select({ id: users.id, email: users.email, name: users.name, passwordHash: users.passwordHash, emailVerifiedAt: users.emailVerifiedAt, authVersion: users.authVersion }).from(users).where(eq(users.email, email)).limit(1)
  return user
}

export async function userExists(email: string) {
  return (await useDb().select({ id: users.id }).from(users).where(eq(users.email, email)).limit(1)).length > 0
}

export async function createUser(input: { email: string, name: string, passwordHash: string }) {
  const db = useDb()
  return db.transaction(async (tx) => {
    const [user] = await tx.insert(users).values(input).returning({ id: users.id, email: users.email, name: users.name, emailVerifiedAt: users.emailVerifiedAt })
    await tx.insert(workspaces).values({ userId: user!.id, name: 'Personal', path: '', pathConfirmed: false })
    return user
  })
}

export async function findUserByEmail(email: string) {
  const [user] = await useDb().select({ id: users.id, email: users.email }).from(users).where(eq(users.email, email)).limit(1)
  return user
}

export async function addVerificationToken(input: { tokenHash: string, userId: string, type: 'password_reset' | 'email_verify', expiresAt: Date }) {
  const db = useDb()
  if (input.type !== 'password_reset') {
    await db.insert(verificationTokens).values(input)
    return
  }

  await db.transaction(async (tx) => {
    await tx.delete(verificationTokens).where(and(eq(verificationTokens.userId, input.userId), eq(verificationTokens.type, 'password_reset')))
    await tx.insert(verificationTokens).values(input)
  })
}

export async function consumePasswordReset(tokenHash: string, passwordHash: string) {
  const db = useDb()
  return db.transaction(async (tx) => {
    const now = new Date()
    const [record] = await tx.update(verificationTokens)
      .set({ consumedAt: now })
      .where(and(
        eq(verificationTokens.tokenHash, tokenHash),
        eq(verificationTokens.type, 'password_reset'),
        isNull(verificationTokens.consumedAt),
        gt(verificationTokens.expiresAt, now)
      ))
      .returning()
    if (!record) return null

    // Invalidate any other outstanding reset links for this account. The
    // normal issue path keeps only one active token, but this also closes the
    // edge case where concurrent reset requests race during issuance.
    await tx.update(verificationTokens)
      .set({ consumedAt: now })
      .where(and(
        eq(verificationTokens.userId, record.userId),
        eq(verificationTokens.type, 'password_reset'),
        isNull(verificationTokens.consumedAt)
      ))

    await tx.update(users).set({
      passwordHash,
      authVersion: sql`${users.authVersion} + 1`,
      updatedAt: now
    }).where(eq(users.id, record.userId))
    return record
  })
}

export async function consumeEmailVerification(tokenHash: string) {
  const db = useDb()
  const [record] = await db.select().from(verificationTokens).where(and(eq(verificationTokens.tokenHash, tokenHash), eq(verificationTokens.type, 'email_verify'))).limit(1)
  if (!record) return null
  const now = new Date()
  await db.transaction(async (tx) => {
    await tx.update(verificationTokens).set({ consumedAt: now }).where(eq(verificationTokens.tokenHash, tokenHash))
    await tx.update(users).set({ emailVerifiedAt: now, updatedAt: now }).where(eq(users.id, record.userId))
  })
  return { record, now }
}
