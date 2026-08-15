import { useDb } from './connection'
import { and, eq } from 'drizzle-orm'
import { users, verificationTokens, workspaces } from '../../database/schema'

export async function findLoginUser(email: string) {
  const [user] = await useDb().select({ id: users.id, email: users.email, name: users.name, passwordHash: users.passwordHash, emailVerifiedAt: users.emailVerifiedAt }).from(users).where(eq(users.email, email)).limit(1)
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
  await useDb().insert(verificationTokens).values(input)
}

export async function consumePasswordReset(tokenHash: string, passwordHash: string) {
  const db = useDb()
  const [record] = await db.select().from(verificationTokens).where(and(eq(verificationTokens.tokenHash, tokenHash), eq(verificationTokens.type, 'password_reset'))).limit(1)
  if (!record) return null
  await db.transaction(async (tx) => {
    await tx.update(verificationTokens).set({ consumedAt: new Date() }).where(eq(verificationTokens.tokenHash, tokenHash))
    await tx.update(users).set({ passwordHash, updatedAt: new Date() }).where(eq(users.id, record.userId))
  })
  return record
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
