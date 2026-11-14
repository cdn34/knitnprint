import { expect, test, type Locator } from '@playwright/test'
import { createServer, type Server } from 'node:http'
import { randomUUID } from 'node:crypto'

const product = {
  id: 'preview-product', slug: 'preview-sweat', title: 'Sweat de teste', description: '', status: 'active', search_keywords: '', categories: [], personalization_area_price_minor: 100,
  variants: [{ id: 'preview-variant', title: 'Rosa', sku: 'PREVIEW', price_minor: 4000, currency: 'EUR', option_values: {}, position: 0, available_quantity: 100, low_stock: false }],
  media: [{ id: 'preview-photo', alt_text: 'Sweat', position: 0, url: '/preview-sweat.svg', thumbnail_url: '/preview-sweat.svg', card_url: '/preview-sweat.svg', detail_url: '/preview-sweat.svg' }],
  personalization: {
    mode: 'photo_text', print_areas: [], text_max_characters: 35, text_min_size: 12, text_max_size: 72, allowed_fonts: ['Montserrat'], allowed_colors: ['#1f4f78'],
    views: [{ id: 'back', label: 'Costas', media_id: 'preview-photo', mode: 'photo_text', print_areas: [{ id: 'one', label: 'Área 1', x: 3400, y: 3500, width: 3200, height: 3000, physical_width_cm: 20, physical_height_cm: 20 }] }],
  },
}
let server: Server
test.beforeAll(async () => {
  server = createServer((request, response) => {
    response.setHeader('content-type', 'application/json')
    const path = request.url?.split('?')[0]
    response.end(JSON.stringify(path === '/api/products/preview-sweat' ? product : path === '/api/products' ? [product] : []))
  })
  await new Promise<void>((resolve, reject) => { server.once('error', reject); server.listen(18082, '127.0.0.1', resolve) })
})
test.afterAll(async () => { await new Promise<void>(resolve => server.close(() => resolve())) })

async function textLayout(locator: Locator) {
  return locator.evaluate(element => {
    const canvas = element.closest('.personalizer-canvas, .personalization-final-preview-canvas')!.getBoundingClientRect()
    const frame = element.parentElement!.getBoundingClientRect()
    const style = getComputedStyle(element)
    const text = element.firstChild!
    const lines = new Set<number>()
    const bands = new Map<number, { top: number; height: number }>()
    let clipped = 0
    for (let index = 0; index < text.textContent!.length; index++) {
      if (/\s/.test(text.textContent![index])) continue
      const range = document.createRange()
      range.setStart(text, index); range.setEnd(text, index + 1)
      const bounds = range.getBoundingClientRect()
      const line = Math.round(bounds.top * 2) / 2
      lines.add(line)
      bands.set(line, { top: (bounds.top - canvas.top) / canvas.height, height: bounds.height / canvas.height })
      if (bounds.top < frame.top - 1 || bounds.bottom > frame.bottom + 1 || bounds.left < frame.left - 1 || bounds.right > frame.right + 1) clipped++
    }
    return { lines: lines.size, clipped, relativeFont: parseFloat(style.fontSize) / canvas.width, bands: [...bands.values()] }
  })
}

test('keeps the full text and identical line wrapping in the editor, preview and saved image', async ({ page }, testInfo) => {
  await page.addInitScript(() => localStorage.setItem('knitnprint-language', 'pt'))
  const uploads = new Map<string, string>()
  let cart: Record<string, unknown> = { id: 'preview-cart', currency: 'EUR', items: [], item_count: 0, subtotal_minor: 0, total_minor: 0 }
  let savedImage: Buffer | undefined
  await page.route('https://fonts.googleapis.com/**', route => route.fulfill({ contentType: 'text/css', body: '' }))
  await page.route('**/preview-sweat.svg', route => route.fulfill({ contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="600" height="600"><rect width="600" height="600" fill="white"/><path d="M180 60L80 160l60 100 50-40v320h220V220l50 40 60-100-100-100z" fill="#f4d1da"/></svg>' }))
  await page.route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname
    if (path === '/api/account/me') return route.fulfill({ json: { id: 'preview-customer', email: 'preview@example.test', first_name: 'Ana', last_name: 'Silva', phone: '', email_verified: true, auth_methods: { password: true, google_email: null }, addresses: [] } })
    if (path === '/api/products/preview-sweat') return route.fulfill({ json: product })
    if (path === '/api/products') return route.fulfill({ json: [product] })
    if (path === '/api/cart') return route.fulfill({ json: cart })
    if (path === '/api/personalization/uploads') {
      const id = randomUUID()
      uploads.set(id, route.request().postDataJSON().filename)
      return route.fulfill({ status: 201, json: { id, upload_url: `http://127.0.0.1:13000/api/test-upload/${id}` } })
    }
    if (path.startsWith('/api/test-upload/')) {
      if (uploads.get(path.split('/').at(-1)!) === 'personalized-preview.png') savedImage = route.request().postDataBuffer()!
      return route.fulfill({ status: 200, body: '' })
    }
    if (path.endsWith('/complete')) return route.fulfill({ json: { id: path.split('/')[4], preview_url: '/preview-sweat.svg' } })
    if (path === '/api/cart/items') {
      cart = { ...cart, item_count: 1, subtotal_minor: 4100, total_minor: 4100, items: [{ id: 'preview-line', product_slug: product.slug, product_title: product.title, variant_title: 'Rosa', quantity: 1, unit_price_minor: 4100, base_unit_price_minor: 4000, personalization_unit_price_minor: 100, line_total_minor: 4100, currency: 'EUR', available: true, customization: route.request().postDataJSON().customization, customization_media_asset_ids: [] }] }
      return route.fulfill({ status: 201, json: cart })
    }
    return route.fulfill({ json: [] })
  })
  await page.goto('/products/preview-sweat/personalize')
  await page.getByRole('textbox', { name: /^O teu texto/ }).fill('Café é sempre uma boa ideia!')
  await page.getByRole('slider', { name: 'Tamanho da letra' }).fill('25')
  const photo = await page.evaluate(() => {
    const canvas = document.createElement('canvas'); canvas.width = 40; canvas.height = 40
    const context = canvas.getContext('2d')!; context.fillStyle = '#326e46'; context.fillRect(0, 0, 40, 40)
    return canvas.toDataURL('image/png').split(',')[1]
  })
  await page.locator('input[type="file"]').setInputFiles({ name: 'photo.png', mimeType: 'image/png', buffer: Buffer.from(photo, 'base64') })
  await expect(page.locator('.personalizer-upload')).toContainText('Trocar fotografia')
  const editor = page.locator('.personalizer-text')
  const savedCanvas = page.locator('.personalizer > div[aria-hidden="true"] .personalization-final-element--text > span')
  const initial = await textLayout(editor)
  await page.getByRole('button', { name: 'Pré-visualizar resultado', exact: true }).click()
  const preview = page.getByRole('dialog').locator('.personalization-final-element--text > span')
  const result = await textLayout(preview)
  expect(result.relativeFont).toBeCloseTo(initial.relativeFont, 4)
  expect(result.lines).toBe(initial.lines)
  expect(initial.clipped).toBe(0)
  expect(result.clipped).toBe(0)
  expect((await textLayout(savedCanvas)).lines).toBe(initial.lines)
  expect((await textLayout(savedCanvas)).clipped).toBe(0)
  await page.screenshot({ path: testInfo.outputPath('complete-preview-text.png') })

  // Changing the available space must scale the design rather than change its line breaks.
  const viewport = page.viewportSize()!
  await page.setViewportSize({ width: viewport.width > 700 ? 960 : 360, height: viewport.height })
  expect((await textLayout(preview)).lines).toBe(initial.lines)
  expect((await textLayout(preview)).clipped).toBe(0)
  await page.getByRole('dialog').getByRole('button', { name: 'Continuar a personalizar' }).click()
  expect((await textLayout(editor)).lines).toBe(initial.lines)
  expect((await textLayout(editor)).clipped).toBe(0)

  const storedLayout = await textLayout(savedCanvas)
  await page.getByRole('button', { name: 'Adicionar ao carrinho', exact: true }).click()
  await expect(page).toHaveURL(/\/products\/?$/)
  await expect(page.getByRole('dialog', { name: 'Carrinho' }).locator('.cart-drawer-item')).toHaveCount(1)
  expect(savedImage).toBeDefined()
  await testInfo.attach('saved-personalization', { body: savedImage!, contentType: 'image/png' })
  const exportedInk = await page.evaluate(async base64 => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode()
    const canvas = document.createElement('canvas'); canvas.width = image.width; canvas.height = image.height
    const context = canvas.getContext('2d')!; context.drawImage(image, 0, 0)
    const { data } = context.getImageData(0, 0, canvas.width, canvas.height)
    const rows = []
    for (let y = 0; y < canvas.height; y++) {
      for (let x = 0; x < canvas.width; x++) {
        const offset = (y * canvas.width + x) * 4
        if (data[offset + 2] > data[offset] + 30 && data[offset + 2] > data[offset + 1] + 20) { rows.push(y); break }
      }
    }
    return { rows, height: canvas.height }
  }, savedImage!.toString('base64'))
  // Every line's central glyph region must contain ink in the actual exported PNG.
  for (const band of storedLayout.bands) {
    const top = (band.top + band.height * 0.3) * exportedInk.height
    const bottom = (band.top + band.height * 0.8) * exportedInk.height
    expect(exportedInk.rows.some(row => row >= top && row <= bottom)).toBe(true)
  }
})
