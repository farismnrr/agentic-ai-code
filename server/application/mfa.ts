import { createHash, randomBytes } from 'node:crypto'

export function generateRecoveryCodes(count = 10) {
  return Array.from({ length: count }, () => {
    const value = randomBytes(8).toString('hex').match(/.{1,4}/g)!.join('-')
    return { value, hash: createHash('sha256').update(value).digest('hex') }
  })
}
