const { chromium } = require('playwright')
const fs = require('fs')
const http = require('http')

const baseUrl = process.env.E2E_BASE_URL || 'http://127.0.0.1:13000'
const artifactDir = process.env.E2E_ARTIFACT_DIR || '/artifacts'
const oauthPort = Number(process.env.E2E_OAUTH_PORT || 4400)

function startOAuthMock() {
  return new Promise((resolve, reject) => {
    const server = http.createServer(async (request, response) => {
      const url = new URL(request.url, `http://127.0.0.1:${oauthPort}`)

      if (request.method === 'GET' && url.pathname === '/authorize') {
        const redirectUri = url.searchParams.get('redirect_uri')
        const state = url.searchParams.get('state')
        if (!redirectUri || !state) {
          response.writeHead(400).end('missing oauth parameters')
          return
        }

        const callback = new URL(redirectUri)
        callback.searchParams.set('code', 'e2e-code')
        callback.searchParams.set('state', state)
        response.writeHead(302, { Location: callback.href }).end()
        return
      }

      if (request.method === 'POST' && url.pathname === '/token') {
        let body = ''
        for await (const chunk of request) body += chunk
        const form = new URLSearchParams(body)

        if (
          form.get('code') !== 'e2e-code'
          || form.get('client_id') !== 'e2e-client'
          || form.get('client_secret') !== 'e2e-secret'
        ) {
          response.writeHead(401).end('invalid token request')
          return
        }

        response.writeHead(200, { 'Content-Type': 'application/json' })
        response.end(JSON.stringify({ access_token: 'e2e-token', token_type: 'bearer' }))
        return
      }

      if (request.method === 'GET' && url.pathname === '/user') {
        if (request.headers.authorization !== 'Bearer e2e-token') {
          response.writeHead(401).end('invalid bearer token')
          return
        }

        response.writeHead(200, { 'Content-Type': 'application/json' })
        response.end(JSON.stringify({
          id: 120432426,
          login: 'farismnrr-e2e',
          avatar_url: null
        }))
        return
      }

      response.writeHead(404).end('not found')
    })

    server.once('error', reject)
    server.listen(oauthPort, '0.0.0.0', () => resolve(server))
  })
}

async function main() {
  fs.mkdirSync(artifactDir, { recursive: true })
  const oauthServer = await startOAuthMock()
  const browser = await chromium.launch({ headless: true })
  const context = await browser.newContext({ viewport: { width: 1536, height: 900 } })
  const page = await context.newPage()

  try {
    const response = await page.goto(baseUrl, { waitUntil: 'networkidle' })
    if (!response || !response.ok()) {
      throw new Error(`page load failed: ${response?.status() ?? 'no response'}`)
    }

    await page.getByText('Masih Awam SSO', { exact: true }).waitFor()
    const signIn = page.getByRole('button', { name: 'Continue with GitHub' })
    await signIn.waitFor()

    const brokenImages = await page.locator('img').evaluateAll(images =>
      images
        .filter(image => !image.complete || image.naturalWidth === 0 || image.naturalHeight === 0)
        .map(image => image.getAttribute('src'))
    )
    if (brokenImages.length) {
      throw new Error(`broken images: ${brokenImages.join(', ')}`)
    }

    const artwork = page.locator('img[src="/assets/security-auth.webp"]')
    await artwork.waitFor({ state: 'visible' })

    await Promise.all([
      page.waitForURL(`${baseUrl}/`),
      signIn.click()
    ])

    await page.getByText('@farismnrr-e2e', { exact: true }).waitFor()
    await page.getByText('Active session', { exact: true }).waitFor()

    const session = await page.request.get(`${baseUrl}/api/session`)
    if (!session.ok()) {
      throw new Error(`session endpoint failed: ${session.status()}`)
    }
    const payload = await session.json()
    if (!payload.authenticated || payload.user?.login !== 'farismnrr-e2e') {
      throw new Error(`unexpected authenticated session: ${JSON.stringify(payload)}`)
    }

    await page.reload({ waitUntil: 'networkidle' })
    await page.getByText('@farismnrr-e2e', { exact: true }).waitFor()

    await page.getByRole('button', { name: 'Sign out' }).click()
    await page.getByRole('button', { name: 'Continue with GitHub' }).waitFor()

    const signedOutSession = await page.request.get(`${baseUrl}/api/session`)
    const signedOutPayload = await signedOutSession.json()
    if (signedOutPayload.authenticated) {
      throw new Error('session remained authenticated after logout')
    }

    await page.screenshot({
      path: `${artifactDir}/sso-auth-home.png`,
      fullPage: true
    })

    console.log('E2E passed: GitHub OAuth, callback, session hydration, and logout')
  } catch (error) {
    await page.screenshot({
      path: `${artifactDir}/sso-auth-failure.png`,
      fullPage: true
    })
    throw error
  } finally {
    await context.close()
    await browser.close()
    await new Promise(resolve => oauthServer.close(resolve))
  }
}

main().catch(error => {
  console.error(error)
  process.exit(1)
})
