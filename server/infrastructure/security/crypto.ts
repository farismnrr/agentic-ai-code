import crypto from 'node:crypto'
import { useRuntimeConfig } from '#imports'

function getSecretKey(): Buffer {
  const config = useRuntimeConfig()
  const key = Buffer.from(config.modelProviderSecretKey, 'hex')
  if (key.length !== 32) {
    throw new Error('NUXT_MODEL_PROVIDER_SECRET_KEY must be set to a 32-byte hex string (generate with `openssl rand -hex 32`)')
  }
  return key
}

export function encryptSecret(text: string): string {
  const key = getSecretKey()
  const iv = crypto.randomBytes(12)
  const cipher = crypto.createCipheriv('aes-256-gcm', key, iv)

  let encrypted = cipher.update(text, 'utf8', 'hex')
  encrypted += cipher.final('hex')

  const authTag = cipher.getAuthTag().toString('hex')
  return `${iv.toString('hex')}:${authTag}:${encrypted}`
}

export function decryptSecret(encryptedData: string): string {
  const key = getSecretKey()
  const parts = encryptedData.split(':')
  if (parts.length !== 3 || !parts[0] || !parts[1] || !parts[2]) {
    throw new Error('Invalid encrypted data format')
  }

  const iv = Buffer.from(parts[0], 'hex')
  const authTag = Buffer.from(parts[1], 'hex')
  const encryptedText = Buffer.from(parts[2], 'hex')

  const decipher = crypto.createDecipheriv('aes-256-gcm', key, iv)
  decipher.setAuthTag(authTag)

  let decrypted = decipher.update(encryptedText, undefined, 'utf8')
  decrypted += decipher.final('utf8')
  return decrypted
}

// `encryptSecret`'s output is always `<12-byte iv hex>:<16-byte authTag
// hex>:<ciphertext hex>` — 24 hex chars, then 32 hex chars, then one or more
// hex chars. A legacy plaintext header value (e.g. `Bearer sk-...`) will
// essentially never collide with this exact shape, so it's a cheap,
// deterministic signature for "is this already-encrypted ciphertext or a
// legacy plaintext value" without needing a new column/format (Plan 031A
// Finding R).
const ENCRYPTED_SECRET_PATTERN = /^[0-9a-f]{24}:[0-9a-f]{32}:[0-9a-f]+$/i

export function isEncryptedSecret(value: string): boolean {
  return ENCRYPTED_SECRET_PATTERN.test(value)
}

/**
 * Custom provider headers frequently carry credentials (`Authorization`,
 * `X-Api-Key`, gateway tokens), so their values are stored encrypted with
 * the same mechanism as API keys — this decrypts them back to plaintext at
 * the point of use (building an outbound request), never for client
 * projection.
 *
 * Rows written before this encryption existed may still hold legacy
 * plaintext values (no `iv:authTag:cipher` shape) — those are passed
 * through as-is rather than fed to `decryptSecret` (which would throw).
 * A value that *looks* encrypted but fails to decrypt (corrupt ciphertext,
 * wrong key) is dropped rather than thrown — callers get one fewer header
 * instead of a crashed request.
 */
export function decryptHeaders(encryptedHeaders: Record<string, string>): Record<string, string> {
  const result: Record<string, string> = {}
  for (const [key, value] of Object.entries(encryptedHeaders)) {
    if (!isEncryptedSecret(value)) {
      result[key] = value
      continue
    }
    try {
      result[key] = decryptSecret(value)
    } catch {
      // Malformed/corrupt encrypted value — fail safe, drop the header.
    }
  }
  return result
}
