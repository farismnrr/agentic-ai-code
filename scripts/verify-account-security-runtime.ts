import { browserSessionFrom, hashSessionSecret, isFreshAuth, issueBrowserAuthSession } from '../server/application/auth-session.ts'
import { generateRecoveryCodes } from '../server/application/mfa.ts'
import { verifyTotpCode } from '../server/infrastructure/security/totp.ts'

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message)
}

const first = issueBrowserAuthSession()
const second = issueBrowserAuthSession()
assert(first.id !== second.id && first.secret !== second.secret, 'browser sessions must be unique')
assert(hashSessionSecret(first.secret) !== first.secret, 'browser secrets must not be persisted raw')
assert(isFreshAuth({ secure: { authSession: first } }), 'new browser sessions must be fresh')
assert(!isFreshAuth({ secure: { authSession: { ...first, freshAuthAt: Date.now() - 10 * 60 * 1000 - 1 } } }), 'fresh authentication must expire')
assert(!browserSessionFrom({ secure: { authSession: { ...first, type: 'api_key' } } }), 'API-key sessions must not be treated as browser sessions')

const codes = generateRecoveryCodes()
assert(codes.length === 10, 'recovery-code set must be bounded')
assert(new Set(codes.map(code => code.value)).size === codes.length, 'recovery codes must be unique')
assert(codes.every(code => /^[a-f0-9]{4}(?:-[a-f0-9]{4}){3}$/.test(code.value)), 'recovery code format must be stable')
assert(codes.every(code => code.hash !== code.value && code.hash.length === 64), 'recovery codes must only persist hashes')

assert(verifyTotpCode('GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ', '287082', 59_000), 'RFC TOTP vector must pass')
assert(!verifyTotpCode('GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ', '287083', 59_000), 'wrong TOTP code must fail')

console.log('account-security-runtime: PASS')
