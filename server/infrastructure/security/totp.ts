import { createHmac, randomBytes, timingSafeEqual } from 'node:crypto'

const BASE32_ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'

export function generateTotpSecret() {
  return encodeBase32(randomBytes(20))
}

export function verifyTotpCode(secret: string, input: string, now = Date.now()) {
  if (!/^\d{6}$/.test(input)) return false
  const counter = Math.floor(now / 1000 / 30)
  for (const offset of [-1, 0, 1]) {
    const expected = totp(secret, counter + offset)
    if (timingSafeEqual(Buffer.from(expected), Buffer.from(input))) return true
  }
  return false
}

export function buildTotpUri(secret: string, email: string, issuer = 'Masih Awam AI Code') {
  const label = `${encodeURIComponent(issuer)}:${encodeURIComponent(email)}`
  return `otpauth://totp/${label}?secret=${secret}&issuer=${encodeURIComponent(issuer)}&algorithm=SHA1&digits=6&period=30`
}

function totp(secret: string, counter: number) {
  const key = decodeBase32(secret)
  const buffer = Buffer.alloc(8)
  buffer.writeBigUInt64BE(BigInt(counter))
  const digest = createHmac('sha1', key).update(buffer).digest()
  const offset = digest[digest.length - 1]! & 0x0f
  const value = (digest[offset]! & 0x7f) << 24
    | digest[offset + 1]! << 16
    | digest[offset + 2]! << 8
    | digest[offset + 3]!
  return String(value % 1_000_000).padStart(6, '0')
}

function encodeBase32(input: Buffer) {
  let bits = 0
  let value = 0
  let output = ''
  for (const byte of input) {
    value = (value << 8) | byte
    bits += 8
    while (bits >= 5) {
      output += BASE32_ALPHABET[(value >>> (bits - 5)) & 31]
      bits -= 5
    }
  }
  if (bits > 0) output += BASE32_ALPHABET[(value << (5 - bits)) & 31]
  return output
}

function decodeBase32(input: string) {
  let bits = 0
  let value = 0
  const output: number[] = []
  for (const char of input.toUpperCase().replace(/=+$/, '')) {
    const index = BASE32_ALPHABET.indexOf(char)
    if (index < 0) throw new Error('Invalid TOTP secret')
    value = (value << 5) | index
    bits += 5
    if (bits >= 8) {
      output.push((value >>> (bits - 8)) & 255)
      bits -= 8
    }
  }
  return Buffer.from(output)
}
