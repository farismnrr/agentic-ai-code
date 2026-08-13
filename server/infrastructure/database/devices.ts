import { useDb } from "./connection"
import { and, eq, isNull } from 'drizzle-orm'
import { userDevices } from '../../database/schema'

/**
 * Narrow persistence lookup: does this user have at least one non-revoked
 * paired relay-agent device. The local-terminal tool-availability policy
 * that consumes this lives in
 * `server/application/chat/local-terminal-policy.ts` (Plan 031A finding G
 * — infrastructure stays a plain persistence lookup, application decides
 * what the pairing state means for the current chat turn).
 */
export async function hasActivePairedDevice(userId: string) {
  const db = useDb()
  const [device] = await db.select({ id: userDevices.id })
    .from(userDevices)
    .where(and(eq(userDevices.userId, userId), isNull(userDevices.revokedAt)))
    .limit(1)
  return Boolean(device)
}
