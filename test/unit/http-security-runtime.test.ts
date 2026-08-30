import assert from 'node:assert/strict'
import test from 'node:test'
import {
  allowsMutationContentType,
  allowsMutationOrigin,
  securityHeadersForPath
} from '../../server/application/http-security.ts'

const base = { method: 'POST', path: '/api/settings', siteUrl: 'https://app.example.com' }

test('cookie-authenticated mutations require same-origin proof', () => {
  assert.equal(allowsMutationOrigin({ ...base, origin: 'https://app.example.com' }), true)
  assert.equal(allowsMutationOrigin({ ...base, origin: 'https://evil.example' }), false)
  assert.equal(allowsMutationOrigin({ ...base, secFetchSite: 'same-origin' }), true)
  assert.equal(allowsMutationOrigin({ ...base, secFetchSite: 'same-site' }), false)
  assert.equal(allowsMutationOrigin({ ...base }), false)
})

test('explicit API bearer calls may omit browser Origin headers', () => {
  assert.equal(allowsMutationOrigin({ ...base, authorization: 'Bearer aic_live_example' }), true)
  assert.equal(allowsMutationOrigin({ ...base, authorization: 'Bearer unrelated' }), false)
})

test('mutation bodies must declare JSON while bodyless mutations remain valid', () => {
  assert.equal(allowsMutationContentType({ ...base, contentType: 'application/json; charset=utf-8' }), true)
  assert.equal(allowsMutationContentType({ ...base, contentType: 'text/plain', contentLength: '4' }), false)
  assert.equal(allowsMutationContentType({ ...base, contentLength: '4' }), false)
  assert.equal(allowsMutationContentType({ ...base, transferEncoding: 'chunked' }), false)
  assert.equal(allowsMutationContentType(base), true)
})

test('safe methods and non-api paths are not mutation-filtered', () => {
  assert.equal(allowsMutationOrigin({ ...base, method: 'GET', origin: 'https://evil.example' }), true)
  assert.equal(allowsMutationContentType({ ...base, path: '/auth/callback', contentType: 'text/plain' }), true)
})

test('sensitive API headers are no-store and frame constrained', () => {
  const auth = securityHeadersForPath('/api/auth/login')
  assert.equal(auth['Cache-Control'], 'no-store')
  assert.match(auth['Content-Security-Policy'], /default-src 'none'/)
  assert.equal(auth['X-Frame-Options'], 'DENY')

  const ordinary = securityHeadersForPath('/api/workspaces')
  assert.equal(ordinary['Cache-Control'], 'no-store')
  assert.equal(ordinary['Cross-Origin-Opener-Policy'], 'same-origin')
})
