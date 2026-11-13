import AxeBuilder from '@axe-core/playwright'
import { expect, test, type Page } from '@playwright/test'
import { createServer, type Server } from 'node:http'

const orderId = '019a5000-0000-7000-8000-000000000001'
const previewId = '019a5000-0000-7000-8000-000000000002'
const photoId = '019a5000-0000-7000-8000-000000000003'
const profile = { id: 'confirmation-customer', email: 'confirmation@example.test', first_name: 'Ana', last_name: 'Silva', phone: '', email_verified: true, addresses: [] }
const address = { recipient_name: 'Ana Silva', line1: 'Rua dos Presentes 1', line2: '', city: 'Lisboa', region: '', postal_code: '1000-001', country_code: 'PT', phone: '' }
const plainLine = { id: 'plain-line', product_title: 'Sweat com capuz', variant_title: 'Rosa · M', sku: 'SWEAT-M', quantity: 1, fulfilled_quantity: 0, unit_price_minor: 4000, line_total_minor: 4000, currency: 'EUR', customization: null, customization_media_asset_id: null, customization_media_asset_ids: [], personalization_context: null, personalization_previews: [], variant_options: { Tamanho: 'M' } }
const personalizedLine = { ...plainLine, id: 'personalized-line', unit_price_minor: 4100, line_total_minor: 4100, customization_media_asset_ids: [photoId], customization: { version: 7, areas: [{ view_id: 'back', view_label: 'Costas', area_id: 'one', area_label: 'Área 1', text: { content: 'Café é sempre uma boa ideia!', font: 'Montserrat', color: '#1f4f78', size: 25 }, photo: { media_id: photoId } }] }, personalization_previews: [{ view_id: 'back', view_label: 'Costas', media_id: previewId }] }
function savedOrder(legacy = false) {
  const custom = legacy ? { ...personalizedLine, customization: { version: 3, text: { content: 'Para a melhor mãe', font: 'Pacifico', color: '#b3232f', size: 24 }, photo: { media_id: photoId } }, personalization_previews: [] } : personalizedLine
  return { id: orderId, order_number: 'KNP-2026-000123', order_status: 'confirmed', payment_status: 'pending', fulfillment_status: 'unfulfilled', currency: 'EUR', subtotal_minor: 8100, discount_minor: 0, discount: null, shipping_minor: 0, shipping: { method_name: 'Shipping not charged', carrier_name: '', transit_hours: 0, amount_minor: 0, currency: 'EUR', country_code: 'PT' }, tax_minor: 0, tax: { rate_basis_points: 0, amount_minor: 0 }, total_minor: 8100, customer: profile, shipping_address: address, lines: [plainLine, custom], payment: { provider: 'manual', status: 'pending', failure_message: null }, refunds: [], operations: {}, fulfillments: [], notifications: [], timeline: [], created_at: '2026-11-04T12:00:00Z' }
}
let server: Server
test.beforeAll(async () => {
  server = createServer((_request, response) => { response.setHeader('content-type', 'application/json'); response.end('[]') })
  await new Promise<void>((resolve, reject) => { server.once('error', reject); server.listen(18084, '127.0.0.1', resolve) })
})
test.afterAll(async () => { await new Promise<void>(resolve => server.close(() => resolve())) })

async function fixture(page: Page, legacy = false) {
  const order = savedOrder(legacy)
  const state = { created: legacy, posts: 0, reads: 0 }
  const cart = { id: 'confirmation-cart', currency: 'EUR', item_count: 2, checkout_ready: true, checkout_quote: 'confirmation-quote', subtotal_minor: 8100, discount_minor: 0, tax_minor: 0, shipping_minor: 0, total_minor: 8100, issues: [], expires_at: '2030-01-01T00:00:00Z', delivery: { ...profile, customer_id: profile.id, address_id: 'confirmation-address', address }, items: order.lines.map((line, index) => ({ ...line, variant_id: `variant-${index}`, product_slug: 'sweat-com-capuz', available: true, available_quantity: 100, base_unit_price_minor: 4000, personalization_unit_price_minor: index ? 100 : 0 })) }
  await page.addInitScript(() => localStorage.setItem('knitnprint-language', 'pt'))
  await page.route('https://fonts.googleapis.com/**', route => route.fulfill({ contentType: 'text/css', body: '' }))
  await page.route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname
    if (path === '/api/account/me') return route.fulfill({ json: profile })
    if (path === '/api/cart') return route.fulfill({ json: state.created ? { ...cart, items: [], item_count: 0, subtotal_minor: 0, total_minor: 0, checkout_ready: false } : cart })
    if (path === '/api/payments/options') return route.fulfill({ json: { stripe: false, manual: true, checkout_suspended: false } })
    if (path === '/api/orders') { state.posts++; state.created = true; return route.fulfill({ status: 201, json: order }) }
    if (path === `/api/orders/${orderId}`) { state.reads++; return route.fulfill({ json: order }) }
    if (path.includes('/personalization/')) {
      if (legacy) return route.fulfill({ status: 404, json: { error: { code: 'not_found', message: 'Not found.' } } })
      return route.fulfill({ contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="600" height="600"><rect width="600" height="600" fill="white"/><path d="M180 60L80 160l60 100 50-40v320h220V220l50 40 60-100-100-100z" fill="#f4d1da"/><path d="M240 240h120v65q0 35-60 35t-60-35z" fill="#5f4652"/><text x="300" y="390" text-anchor="middle" font-family="sans-serif" font-weight="bold" font-size="20" fill="#1f4f78">Café é sempre</text><text x="300" y="420" text-anchor="middle" font-family="sans-serif" font-weight="bold" font-size="20" fill="#1f4f78">uma boa ideia!</text></svg>' })
    }
    return route.fulfill({ json: [] })
  })
  return state
}

test('shows which sweat was personalized, its text and saved result after checkout and reload', async ({ page }, testInfo) => {
  const state = await fixture(page)
  await page.goto('/cart')
  await page.getByRole('button', { name: 'Finalizar encomenda', exact: true }).click()
  await expect(page).toHaveURL(new RegExp(`order_id=${orderId}`))
  const lines = page.locator('.order-confirmation-line')
  await expect(lines).toHaveCount(2)
  await expect(lines.first().locator('.order-line-personalization')).toHaveCount(0)
  const custom = lines.nth(1)
  await expect(custom.locator('.order-personalized-badge')).toHaveText('Personalizado')
  await expect(custom.locator('.order-personalization-text')).toContainText('Café é sempre uma boa ideia!')
  await expect(custom.locator('.order-personalization-areas dt')).toHaveText('Costas · Área 1')
  await expect(custom.locator('.order-personalization-photo')).toHaveText('Fotografia adicionada')
  await expect(custom.locator('.order-personalization-style')).toContainText('Montserrat')
  await expect(custom.locator('.order-personalization-style')).toContainText('Azul')
  await expect(custom.locator('.order-personalization-preview img')).toHaveAttribute('src', `/api/orders/${orderId}/personalization/${previewId}`)
  await custom.scrollIntoViewIfNeeded()
  await expect(custom.locator('img')).toHaveJSProperty('naturalWidth', 600)
  await expect(custom.locator('.order-confirmation-line-heading')).toContainText(/41,00\s*€/)
  expect((await new AxeBuilder({ page }).include('.order-confirmation').analyze()).violations).toEqual([])
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
  await page.locator('.order-confirmation-lines').screenshot({ path: testInfo.outputPath('order-personalization.png') })
  await page.reload()
  await expect(custom.locator('.order-personalization-text')).toContainText('Café é sempre uma boa ideia!')
  await expect(page.locator('.order-personalized-badge')).toHaveCount(1)
  expect(state.posts).toBe(1)
  expect(state.reads).toBeGreaterThan(0)
})

test('keeps legacy personalization readable when an old image is unavailable', async ({ page }) => {
  const state = await fixture(page, true)
  await page.goto(`/cart?order_id=${orderId}&source=account`)
  await expect(page.getByRole('heading', { name: 'A tua encomenda em resumo' })).toBeVisible()
  const custom = page.locator('.order-confirmation-line').nth(1)
  await expect(custom.locator('.order-personalized-badge')).toHaveText('Personalizado')
  await expect(custom.locator('.order-personalization-text')).toContainText('Para a melhor mãe')
  await expect(custom.locator('.order-personalization-style')).toContainText('Pacifico')
  await expect(custom.locator('.order-personalization-style')).toContainText('Vermelho')
  await custom.scrollIntoViewIfNeeded()
  await expect(custom.locator('.order-personalization-unavailable')).toContainText('Os detalhes da personalização continuam guardados')
  await expect(custom.locator('.order-personalization-preview img')).toHaveCount(0)
  await expect(page.locator('.order-confirmation-line').first().locator('.order-personalized-badge')).toHaveCount(0)
  expect(state.posts).toBe(0)
})
