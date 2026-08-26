import { createHash } from 'node:crypto'

export type PasswordScreeningResult = 'clear' | 'breached' | 'unavailable'

const SCREENING_TIMEOUT_MS = 2_000

/**
 * Privacy-preserving Have I Been Pwned range lookup. Only the first five
 * characters of a SHA-1 password digest leave the process; the full password
 * and full digest suffix never enter a URL, header, log, or telemetry field.
 */
export async function screenPassword(password: string): Promise<PasswordScreeningResult> {
  const digest = createHash('sha1').update(password, 'utf8').digest('hex').toUpperCase()
  const prefix = digest.slice(0, 5)
  const suffix = digest.slice(5)
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), SCREENING_TIMEOUT_MS)
  try {
    const response = await fetch(`https://api.pwnedpasswords.com/range/${prefix}`, {
      headers: { 'Add-Padding': 'true' },
      signal: controller.signal
    })
    if (!response.ok) return 'unavailable'
    const body = await response.text()
    if (body.length > 256_000) return 'unavailable'
    return body.split('\n').some(line => line.split(':', 1)[0]?.trim() === suffix) ? 'breached' : 'clear'
  } catch {
    return 'unavailable'
  } finally {
    clearTimeout(timeout)
  }
}
