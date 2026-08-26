import { createCipheriv, createDecipheriv, createHash, randomBytes } from 'node:crypto'

const PREFIX = 'activity-v1'

function keyFromSecret(secret: string) {
  return createHash('sha256').update('ai-code:workspace-activity:v1:').update(secret).digest()
}

export function encryptActivityPayload(plaintext: string, secret: string, aad: string) {
  const iv = randomBytes(12)
  const cipher = createCipheriv('aes-256-gcm', keyFromSecret(secret), iv)
  cipher.setAAD(Buffer.from(aad))
  const ciphertext = Buffer.concat([cipher.update(plaintext, 'utf8'), cipher.final()])
  return `${PREFIX}.${iv.toString('base64url')}.${cipher.getAuthTag().toString('base64url')}.${ciphertext.toString('base64url')}`
}

export function decryptActivityPayload(envelope: string, secret: string, aad: string) {
  const [prefix, ivText, tagText, ciphertextText] = envelope.split('.')
  if (prefix !== PREFIX || !ivText || !tagText || !ciphertextText) throw new Error('Invalid activity payload')
  try {
    const decipher = createDecipheriv('aes-256-gcm', keyFromSecret(secret), Buffer.from(ivText, 'base64url'))
    decipher.setAAD(Buffer.from(aad))
    decipher.setAuthTag(Buffer.from(tagText, 'base64url'))
    return Buffer.concat([decipher.update(Buffer.from(ciphertextText, 'base64url')), decipher.final()]).toString('utf8')
  } catch {
    throw new Error('Invalid activity payload')
  }
}
