import { chromium } from '@playwright/test'
import { readFile, writeFile } from 'node:fs/promises'

const source = await readFile(new URL('../images/logo.png', import.meta.url))
const sourceUrl = `data:image/png;base64,${source.toString('base64')}`
const browser = await chromium.launch({ headless: true })
const page = await browser.newPage()

async function render({ crop, height, output, quality, square = false }) {
  const encoded = await page.evaluate(
    async ({ crop, height, quality, sourceUrl, square }) => {
      const image = new Image()
      image.src = sourceUrl
      await image.decode()

      const renderedWidth = Math.round((crop.width / crop.height) * height)
      const width = square ? height : renderedWidth
      const canvas = document.createElement('canvas')
      canvas.width = width
      canvas.height = height
      const context = canvas.getContext('2d')
      if (!context) throw new Error('Canvas 2D context is unavailable')

      context.drawImage(
        image,
        crop.x,
        crop.y,
        crop.width,
        crop.height,
        square ? Math.round((width - renderedWidth) / 2) : 0,
        0,
        renderedWidth,
        height,
      )
      return canvas.toDataURL('image/webp', quality)
    },
    { crop, height, quality, sourceUrl, square },
  )

  const bytes = Buffer.from(encoded.split(',')[1], 'base64')
  await writeFile(new URL(output, import.meta.url), bytes)
  console.log(`wrote ${output} (${bytes.length} bytes)`)
}

await render({
  crop: { x: 18, y: 335, width: 1500, height: 390 },
  height: 195,
  quality: 0.84,
  output: '../apps/storefront/public/knitprint-wordmark.webp',
})
await render({
  crop: { x: 18, y: 335, width: 1500, height: 390 },
  height: 195,
  quality: 0.84,
  output: '../apps/admin/public/knitprint-wordmark.webp',
})
await render({
  crop: { x: 15, y: 345, width: 285, height: 300 },
  height: 256,
  quality: 0.86,
  square: true,
  output: '../apps/storefront/public/knitprint-mark.webp',
})
await render({
  crop: { x: 15, y: 345, width: 285, height: 300 },
  height: 256,
  quality: 0.86,
  square: true,
  output: '../apps/admin/public/knitprint-mark.webp',
})

await browser.close()
