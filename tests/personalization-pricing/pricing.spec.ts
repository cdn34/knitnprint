import { expect, test } from '@playwright/test'
import { randomUUID } from 'node:crypto'
import { createServer, type Server } from 'node:http'

const area = (id: string, label: string, y: number) => ({ id, label, x: 1500, y, width: 7000, height: 2500, physical_width_cm: 20, physical_height_cm: 10 })
const product = {
  id: 'pricing-product', slug: 'pricing-shirt', title: 'Pricing shirt', description: '', status: 'active', search_keywords: '', categories: [],
  personalization_area_price_minor: 100,
  variants: [{ id: 'pricing-variant', title: 'Default', sku: 'PRICING', price_minor: 2000, currency: 'EUR', option_values: {}, position: 0, available_quantity: 100, low_stock: false }],
  media: [{ id: 'product-photo', alt_text: 'Product', position: 0, url: '/pricing-product.svg', thumbnail_url: '/pricing-product.svg', card_url: '/pricing-product.svg', detail_url: '/pricing-product.svg' }],
  personalization: {
    mode: 'photo_text', print_areas: [], text_max_characters: 35, text_min_size: 12, text_max_size: 72, allowed_fonts: ['Roboto'], allowed_colors: ['#111111'],
    views: [
      { id: 'front', label: 'Frente', media_id: 'product-photo', mode: 'photo_text', print_areas: [area('one', 'Área 1', 1800), area('two', 'Área 2', 5000)] },
      { id: 'back', label: 'Costas', media_id: 'product-photo', mode: 'photo_text', print_areas: [area('one', 'Área 1', 1800)] },
    ],
  },
}

let api: Server
test.beforeAll(async () => {
  // The isolated storefront uses this fixture for server-rendered catalog reads.
  api = createServer((request, response) => {
    response.setHeader('content-type', 'application/json')
    const path = request.url?.split('?')[0]
    response.end(JSON.stringify(path === '/api/products/pricing-shirt' ? product : path === '/api/products' ? [product] : []))
  })
  await new Promise<void>((resolve, reject) => {
    api.once('error', reject)
    api.listen(18082, '127.0.0.1', resolve)
  })
})
test.afterAll(async () => { await new Promise<void>(resolve => api.close(() => resolve())) })

test('counts filled areas once, previews the total and shows the cart surcharge', async ({ page }, testInfo) => {
  await page.addInitScript(() => localStorage.setItem('knitnprint-language', 'pt'))
  const profile = { id: 'pricing-customer', email: 'pricing@example.test', first_name: 'Pricing', last_name: 'Tester', phone: '', email_verified: true, auth_methods: { password: true, google_email: null }, addresses: [] }
  let submitted: { customization: { areas: Array<{ photo?: unknown; text?: unknown }> } } | undefined
  let cart: Record<string, unknown> = { id: 'pricing-cart', currency: 'EUR', items: [], item_count: 0, subtotal_minor: 0, discount_minor: 0, tax_minor: 0, shipping_minor: null, total_minor: 0, issues: [], checkout_ready: false, expires_at: '2030-01-01T00:00:00Z' }
  await page.route('https://fonts.googleapis.com/**', route => route.fulfill({ contentType: 'text/css', body: '' }))
  await page.route('**/pricing-product.svg', route => route.fulfill({ contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="600" height="700"><rect width="600" height="700" fill="#fff"/><path d="M180 100L80 200l60 100 50-40v350h220V260l50 40 60-100-100-100z" fill="#f4d1da"/></svg>' }))
  await page.route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname
    if (path === '/api/account/me') return route.fulfill({ json: profile })
    if (path === '/api/products/pricing-shirt') return route.fulfill({ json: product })
    if (path === '/api/products') return route.fulfill({ json: [product] })
    if (path === '/api/cart') return route.fulfill({ json: cart })
    if (path === '/api/payments/options') return route.fulfill({ json: { stripe: false, manual: true, checkout_suspended: false } })
    if (path === '/api/personalization/uploads') {
      const id = randomUUID()
      return route.fulfill({ status: 201, json: { id, upload_url: `http://127.0.0.1:13000/api/test-upload/${id}` } })
    }
    if (path.startsWith('/api/test-upload/')) return route.fulfill({ status: 200, body: '' })
    if (path.startsWith('/api/personalization/uploads/') && path.endsWith('/complete')) return route.fulfill({ json: { id: path.split('/')[4], preview_url: '/pricing-product.svg' } })
    if (path === '/api/cart/items') {
      submitted = route.request().postDataJSON()
      cart = { ...cart, item_count: 1, subtotal_minor: 2200, total_minor: 2200, items: [{
        id: 'pricing-line', variant_id: 'pricing-variant', product_slug: product.slug, product_title: product.title, variant_title: 'Default', sku: 'PRICING', quantity: 1,
        unit_price_minor: 2200, base_unit_price_minor: 2000, personalization_unit_price_minor: 200, line_total_minor: 2200, currency: 'EUR', available: true, available_quantity: 100,
        customization: submitted!.customization, customization_media_asset_ids: [],
      }] }
      return route.fulfill({ status: 201, json: cart })
    }
    return route.fulfill({ json: [] })
  })
  await page.goto('/products/pricing-shirt/personalize')
  const summary = page.locator('.personalization-checkout-bar .personalization-price-summary')
  const total = summary.locator('strong')
  await expect(total).toHaveText(/20,00\s*€/)
  await page.getByRole('textbox', { name: /^O teu texto/ }).fill('Olá')
  await expect(total).toHaveText(/21,00\s*€/)
  const photo = await page.evaluate(() => {
    const canvas = document.createElement('canvas')
    canvas.width = 40; canvas.height = 40
    const context = canvas.getContext('2d')!
    context.fillStyle = '#9c5263'; context.fillRect(0, 0, 40, 40)
    return canvas.toDataURL('image/png').split(',')[1]
  })
  await page.locator('input[type="file"]').setInputFiles({ name: 'photo.png', mimeType: 'image/png', buffer: Buffer.from(photo, 'base64') })
  await expect(page.locator('.personalizer-upload')).toContainText('Trocar fotografia')
  await expect(total).toHaveText(/21,00\s*€/)
  await page.locator('.personalizer-area-switcher').getByRole('button', { name: '2 Área 2' }).click()
  await page.getByRole('textbox', { name: /^O teu texto/ }).fill('Segunda área')
  await expect(total).toHaveText(/22,00\s*€/)
  await page.getByRole('tab', { name: '2 Costas' }).click()
  await page.getByRole('textbox', { name: /^O teu texto/ }).fill('Costas')
  await expect(total).toHaveText(/23,00\s*€/)
  await page.getByRole('textbox', { name: /^O teu texto/ }).fill('')
  await expect(total).toHaveText(/22,00\s*€/)
  await page.getByRole('tab', { name: '1 Frente' }).click()
  await page.getByRole('button', { name: 'Pré-visualizar resultado', exact: true }).click()
  await expect(page.getByRole('dialog').locator('.personalization-price-summary strong')).toHaveText(/22,00\s*€/)
  await expect(page.getByRole('dialog')).toBeInViewport({ ratio: 1 })
  await expect(page.getByRole('dialog').getByRole('button', { name: 'Adicionar ao carrinho', exact: true })).toBeInViewport({ ratio: 1 })
  await page.screenshot({ path: testInfo.outputPath('personalization-price.png') })
  await expect(page.getByRole('dialog')).toBeInViewport({ ratio: 1 })
  await page.getByRole('dialog').getByRole('button', { name: 'Adicionar ao carrinho', exact: true }).click()
  const confirmation = page.getByRole('alertdialog')
  await expect(confirmation.getByRole('heading')).toHaveText('Tens a certeza que já completaste a tua personalização?')
  await confirmation.getByRole('button', { name: 'Continuar a editar' }).click()
  expect(submitted).toBeUndefined()
  await page.getByRole('button', { name: 'Adicionar ao carrinho', exact: true }).click()
  await confirmation.getByRole('button', { name: 'Adicionar ao carrinho', exact: true }).click()
  await expect(page).toHaveURL(/\/products\/?$/)
  const drawer = page.getByRole('dialog', { name: 'Carrinho' })
  await expect(drawer.locator('.cart-drawer-item')).toHaveCount(1)
  await expect(drawer.locator('.cart-drawer-price-breakdown')).toHaveText(/20,00\s*€.*2,00\s*€/)
  expect(submitted!.customization.areas).toHaveLength(2)
  expect(submitted!.customization.areas[0]).toHaveProperty('photo')
  expect(submitted!.customization.areas[0]).toHaveProperty('text')
  await drawer.getByRole('link', { name: 'Ver carrinho e finalizar', exact: true }).click()
  await expect(page.locator('.cart-item-price-breakdown')).toHaveText(/20,00\s*€.*2,00\s*€/)
  await expect(page.locator('.cart-total')).toContainText(/22,00\s*€/)
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
})
