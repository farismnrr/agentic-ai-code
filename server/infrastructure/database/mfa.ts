import { and, eq, isNull } from 'drizzle-orm'
import { mfaFactors, mfaRecoveryCodes } from '../../database/schema'
import { useDb } from './connection'

export async function createFactor(userId: string, secretEncrypted: string) {
  const [factor] = await useDb().insert(mfaFactors).values({ userId, type: 'totp', secretEncrypted }).returning({
    id: mfaFactors.id,
    createdAt: mfaFactors.createdAt
  })
  return factor
}

export async function findFactor(userId: string, id: string) {
  const [factor] = await useDb().select({
    id: mfaFactors.id,
    secretEncrypted: mfaFactors.secretEncrypted,
    confirmedAt: mfaFactors.confirmedAt,
    revokedAt: mfaFactors.revokedAt
  }).from(mfaFactors).where(and(eq(mfaFactors.id, id), eq(mfaFactors.userId, userId))).limit(1)
  return factor
}

export async function confirmFactor(userId: string, id: string) {
  const [factor] = await useDb().update(mfaFactors).set({ confirmedAt: new Date() }).where(and(
    eq(mfaFactors.id, id),
    eq(mfaFactors.userId, userId),
    isNull(mfaFactors.confirmedAt),
    isNull(mfaFactors.revokedAt)
  )).returning({ id: mfaFactors.id })
  return factor
}

export async function revokeFactor(userId: string, id: string) {
  return useDb().update(mfaFactors).set({ revokedAt: new Date() }).where(and(
    eq(mfaFactors.id, id),
    eq(mfaFactors.userId, userId),
    isNull(mfaFactors.revokedAt)
  )).returning({ id: mfaFactors.id })
}

export async function listFactors(userId: string) {
  return useDb().select({ id: mfaFactors.id, type: mfaFactors.type, createdAt: mfaFactors.createdAt, confirmedAt: mfaFactors.confirmedAt }).from(mfaFactors).where(and(
    eq(mfaFactors.userId, userId),
    isNull(mfaFactors.revokedAt)
  ))
}

export async function replaceRecoveryCodes(userId: string, codeHashes: string[]) {
  return useDb().transaction(async (tx) => {
    await tx.delete(mfaRecoveryCodes).where(eq(mfaRecoveryCodes.userId, userId))
    return tx.insert(mfaRecoveryCodes).values(codeHashes.map(codeHash => ({ userId, codeHash }))).returning({ id: mfaRecoveryCodes.id })
  })
}

export async function consumeRecoveryCode(userId: string, codeHash: string) {
  const [code] = await useDb().update(mfaRecoveryCodes).set({ usedAt: new Date() }).where(and(
    eq(mfaRecoveryCodes.userId, userId),
    eq(mfaRecoveryCodes.codeHash, codeHash),
    isNull(mfaRecoveryCodes.usedAt)
  )).returning({ id: mfaRecoveryCodes.id })
  return code
}
