import { randomBytes, createHash } from 'node:crypto'

export function generateToken() {
  const token = randomBytes(32).toString('hex')
  const hash = createHash('sha256').update(token).digest('hex')
  return { token, hash }
}

export function hashToken(token: string) {
  return createHash('sha256').update(token).digest('hex')
}
