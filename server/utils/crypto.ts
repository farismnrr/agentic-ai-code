import crypto from 'node:crypto'
import { useRuntimeConfig } from '#imports'

export function encryptSecret(text: string): string {
  const config = useRuntimeConfig()
  const key = Buffer.from(config.modelProviderSecretKey, 'hex')
  const iv = crypto.randomBytes(12)
  const cipher = crypto.createCipheriv('aes-256-gcm', key, iv)

  let encrypted = cipher.update(text, 'utf8', 'hex')
  encrypted += cipher.final('hex')

  const authTag = cipher.getAuthTag().toString('hex')
  return `${iv.toString('hex')}:${authTag}:${encrypted}`
}

export function decryptSecret(encryptedData: string): string {
  const config = useRuntimeConfig()
  const key = Buffer.from(config.modelProviderSecretKey, 'hex')
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
