import { useDb } from './connection'
import { and, eq, gt, isNotNull, isNull, sql } from 'drizzle-orm'
import { authSessions, mfaFactors, users, verificationTokens, workspaces } from '../../database/schema'

export async function findLoginUser(email: string) {
  const [user] = await useDb().select({ id: users.id, email: users.email, name: users.name, passwordHash: users.passwordHash, emailVerifiedAt: users.emailVerifiedAt, authVersion: users.authVersion, role: users.role }).from(users).where(eq(users.email, email)).limit(1)
  return user
}

export async function findUserForReauth(userId: string) {
  const [user] = await useDb().select({ id: users.id, email: users.email, name: users.name, passwordHash: users.passwordHash, authVersion: users.authVersion, role: users.role }).from(users).where(eq(users.id, userId)).limit(1)
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
    await tx.update(authSessions).set({ revokedAt: now }).where(and(eq(authSessions.userId, record.userId), isNull(authSessions.revokedAt)))
    return record
  })
}

export async function consumeEmailVerification(tokenHash: string) {
  const db = useDb()
  return db.transaction(async (tx) => {
    const now = new Date()
    const [record] = await tx.update(verificationTokens)
      .set({ consumedAt: now })
      .where(and(
        eq(verificationTokens.tokenHash, tokenHash),
        eq(verificationTokens.type, 'email_verify'),
        isNull(verificationTokens.consumedAt),
        gt(verificationTokens.expiresAt, now)
      ))
      .returning()
    if (!record) return null
    await tx.update(users).set({ emailVerifiedAt: now, updatedAt: now }).where(eq(users.id, record.userId))
    return { record, now }
  })
}

export async function resendEmailVerification(userId: string, input: { tokenHash: string, expiresAt: Date }) {
  const db = useDb()
  return db.transaction(async (tx) => {
    await tx.delete(verificationTokens).where(and(eq(verificationTokens.userId, userId), eq(verificationTokens.type, 'email_verify')))
    return tx.insert(verificationTokens).values({ ...input, userId, type: 'email_verify' }).returning({ tokenHash: verificationTokens.tokenHash })
  })
}

export async function requestEmailChange(userId: string, input: { email: string, tokenHash: string, expiresAt: Date }) {
  const [updated] = await useDb().update(users).set({
    pendingEmail: input.email,
    pendingEmailTokenHash: input.tokenHash,
    pendingEmailExpiresAt: input.expiresAt,
    updatedAt: new Date()
  }).where(eq(users.id, userId)).returning({ id: users.id, email: users.email, pendingEmail: users.pendingEmail })
  return updated
}

export async function consumeEmailChange(tokenHash: string) {
  const db = useDb()
  return db.transaction(async (tx) => {
    const now = new Date()
    const [pending] = await tx.select({ id: users.id, email: users.email, pendingEmail: users.pendingEmail }).from(users).where(and(
      eq(users.pendingEmailTokenHash, tokenHash),
      isNotNull(users.pendingEmail),
      isNotNull(users.pendingEmailExpiresAt),
      gt(users.pendingEmailExpiresAt, now)
    )).limit(1)
    if (!pending?.pendingEmail) return null
    const [updated] = await tx.update(users).set({
      email: pending.pendingEmail,
      pendingEmail: null,
      pendingEmailTokenHash: null,
      pendingEmailExpiresAt: null,
      authVersion: sql`${users.authVersion} + 1`,
      updatedAt: now
    }).where(and(
      eq(users.id, pending.id),
      eq(users.pendingEmailTokenHash, tokenHash),
      isNotNull(users.pendingEmailExpiresAt),
      gt(users.pendingEmailExpiresAt, now)
    )).returning({ id: users.id })
    if (!updated) return null
    await tx.update(authSessions).set({ revokedAt: now }).where(and(eq(authSessions.userId, pending.id), isNull(authSessions.revokedAt)))
    return { ...updated, oldEmail: pending.email, newEmail: pending.pendingEmail, now }
  })
}

export async function hasActiveMfa(userId: string) {
  const [factor] = await useDb().select({ id: mfaFactors.id }).from(mfaFactors).where(and(
    eq(mfaFactors.userId, userId),
    eq(mfaFactors.type, 'totp'),
    isNull(mfaFactors.revokedAt),
    isNotNull(mfaFactors.confirmedAt)
  )).limit(1)
  return Boolean(factor)
}
