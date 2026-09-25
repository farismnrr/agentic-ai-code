const { chromium } = require('playwright')
const fs = require('fs')

const baseUrl = process.env.E2E_BASE_URL || 'http://127.0.0.1:13000'
const artifactDir = process.env.E2E_ARTIFACT_DIR || '/artifacts'

async function main() {
  fs.mkdirSync(artifactDir, { recursive: true })

  const browser = await chromium.launch({ headless: true })
  const page = await browser.newPage({ viewport: { width: 1536, height: 900 } })

  try {
    const response = await page.goto(baseUrl, { waitUntil: 'networkidle' })
    if (!response || !response.ok()) {
      throw new Error(`page load failed: ${response?.status() ?? 'no response'}`)
    }

    await page.getByText('Masih Awam SSO', { exact: true }).waitFor()
    await page.getByRole('button', { name: 'Continue with GitHub' }).waitFor()

    const brokenImages = await page.locator('img').evaluateAll(images =>
      images
        .filter(image => !image.complete || image.naturalWidth === 0 || image.naturalHeight === 0)
        .map(image => image.getAttribute('src'))
    )
    if (brokenImages.length) {
      throw new Error(`broken images: ${brokenImages.join(', ')}`)
    }

    const artwork = page.locator('img[src="/assets/security-auth.png"]')
    await artwork.waitFor({ state: 'visible' })

    const artworkStats = await artwork.evaluate(image => {
      const canvas = document.createElement('canvas')
      canvas.width = image.naturalWidth
      canvas.height = image.naturalHeight

      const context = canvas.getContext('2d')
      if (!context) throw new Error('2d canvas unavailable')

      context.drawImage(image, 0, 0)
      const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data
      const step = 16
      let visible = 0
      let samples = 0

      for (let y = 0; y < canvas.height; y += step) {
        for (let x = 0; x < canvas.width; x += step) {
          const alpha = pixels[(y * canvas.width + x) * 4 + 3]
          samples += 1
          if (alpha > 32) visible += 1
        }
      }

      return {
        naturalWidth: image.naturalWidth,
        naturalHeight: image.naturalHeight,
        visibleRatio: visible / samples
      }
    })

    if (artworkStats.naturalWidth !== 1086 || artworkStats.naturalHeight !== 1448) {
      throw new Error(`unexpected artwork dimensions: ${artworkStats.naturalWidth}x${artworkStats.naturalHeight}`)
    }

    if (artworkStats.visibleRatio < 0.25) {
      throw new Error(`artwork is mostly transparent/blank: visible ratio ${artworkStats.visibleRatio.toFixed(3)}`)
    }

    await page.screenshot({
      path: `${artifactDir}/sso-auth-home.png`,
      fullPage: true
    })

    console.log('E2E passed', artworkStats)
  } catch (error) {
    await page.screenshot({
      path: `${artifactDir}/sso-auth-failure.png`,
      fullPage: true
    })
    throw error
  } finally {
    await browser.close()
  }
}

main().catch(error => {
  console.error(error)
  process.exit(1)
})
