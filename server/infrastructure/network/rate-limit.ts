/**
 * Simple in-memory rate limiter for auth endpoints.
 *
 * Single-node only — appropriate for the current deployment model. If the app
 * ever runs behind multiple workers, replace this with a Redis-backed counter.
 *
 * The limiter allows `maxAttempts` within `windowMs`. On exceeding the limit
 * it returns an error and the caller should respond 429.
 *
 * Keys are arbitrary strings — use `ip:email` for login/register so an
 * attacker can't just rotate IPs for a dictionary attack on one account, and
 * can't rotate email addresses from a single IP.
 */

interface Bucket {
  count: number
  resetAt: number
}

const store = new Map<string, Bucket>()

/** Clean up expired buckets every 5 minutes to prevent unbounded growth. */
setInterval(() => {
  const now = Date.now()
  for (const [key, bucket] of store) {
    if (bucket.resetAt < now) store.delete(key)
  }
}, 5 * 60 * 1000)

export interface RateLimitOptions {
  /** Key to track — e.g. `login:127.0.0.1:user@example.com`. */
  key: string
  /** Max requests before throttling. Default 10. */
  maxAttempts?: number
  /** Window in milliseconds. Default 15 minutes. */
  windowMs?: number
}

export interface RateLimitResult {
  limited: boolean
  /** Seconds until the window resets. Only present when limited === true. */
  retryAfter?: number
}

export function rateLimit({ key, maxAttempts = 10, windowMs = 15 * 60 * 1000 }: RateLimitOptions): RateLimitResult {
  const now = Date.now()
  let bucket = store.get(key)

  if (!bucket || bucket.resetAt < now) {
    bucket = { count: 0, resetAt: now + windowMs }
    store.set(key, bucket)
  }

  bucket.count++

  if (bucket.count > maxAttempts) {
    return {
      limited: true,
      retryAfter: Math.ceil((bucket.resetAt - now) / 1000)
    }
  }

  return { limited: false }
}
