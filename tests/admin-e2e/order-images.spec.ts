import AxeBuilder from '@axe-core/playwright'
import { expect, test, type Page } from '@playwright/test'

const orderId = '01900000-0000-7000-8000-000000000001'
const productPhoto = '01900000-0000-7000-8000-000000000002'
const frontPreview = '01900000-0000-7000-8000-000000000003'
const backPreview = '01900000-0000-7000-8000-000000000004'
const originalPhoto = '01900000-0000-7000-8000-000000000005'
const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a9X0AAAAASUVORK5CYII=', 'base64')
const svg = (caption: string) => `<svg xmlns="http://www.w3.org/2000/svg" width="600" height="700"><rect width="600" height="700" fill="white"/><path d="M180 60L60 190l65 100 65-50v390h220V240l65 50 65-100L420 60z" fill="#f4d1da"/><text x="300" y="370" text-anchor="middle" font-family="sans-serif" font-size="20" fill="#1f4f78">${caption}</text></svg>`

function orderFixture(legacy = false) {
  const plain = {
    id: 'plain-line', product_title: 'Sweat com capuz', variant_title: 'Rosa · M', sku: 'SWEAT-M',
    quantity: 2, fulfilled_quantity: 0, unit_price_minor: 4000, line_total_minor: 8000, currency: 'EUR',
    customization: null, customization_media_asset_id: null, customization_media_asset_ids: [],
    personalization_context: legacy ? null : { version: 1, views: [{ media_id: productPhoto }] },
    personalization_previews: [], variant_options: { Tamanho: 'M' },
  }
  const customized = {
    ...plain, id: 'personalized-line', quantity: 1, unit_price_minor: 4200, line_total_minor: 4200,
    customization_media_asset_ids: [originalPhoto],
    customization: { version: 7, areas: ['Frente', 'Costas'].map((label, index) => ({
      view_id: index ? 'back' : 'front', view_label: label, area_id: 'print', area_label: 'Peito',
      print_width_cm: 20, print_height_cm: 20,
      text: { content: index ? 'Feito para ti' : 'Café é sempre uma boa ideia!', font: 'Montserrat', color: '#1f4f78', size: 22, x: 5, y: 50, width: 90, height: 30 },
      photo: index ? null : { media_id: originalPhoto, x: 10, y: 5, width: 70, height: 40 },
    })) },
    personalization_context: { version: 1, views: ['Frente', 'Costas'].map((label, index) => ({
      id: index ? 'back' : 'front', label, media_id: productPhoto,
      print_areas: [{ id: 'print', x: 2500, y: 2500, width: 5000, height: 5000 }],
    })) },
    personalization_previews: legacy ? [] : [
      { view_id: 'front', view_label: 'Frente', media_id: frontPreview },
      { view_id: 'back', view_label: 'Costas', media_id: backPreview },
    ],
  }
  return {
    id: orderId, order_number: 'KP-2026-000123', order_status: 'confirmed', payment_status: 'paid',
    fulfillment_status: 'unfulfilled', currency: 'EUR', subtotal_minor: 12200, discount_minor: 0, discount: null,
    shipping_minor: 0, shipping: { method_name: 'Entrega', amount_minor: 0 },
    tax_minor: 0, tax: { rate_basis_points: 0, amount_minor: 0 }, total_minor: 12200,
    customer: { first_name: 'Marta', last_name: 'Silva', email: 'marta@example.test', phone: '' },
    shipping_address: { recipient_name: 'Marta Silva', line1: 'Rua da Loja, 1', line2: '', postal_code: '1000-001', city: 'Lisboa', country_code: 'PT' },
    lines: [plain, customized], payment: { provider: 'manual', status: 'paid', attempts: [], history: [] },
    operations: { can_cancel: false, can_refund: false }, refunds: [], fulfillments: [], notifications: [], timeline: [],
    created_at: '2026-11-05T09:00:00Z',
  }
}

async function fixture(page: Page, options: { legacy?: boolean; failBack?: boolean } = {}) {
  const order = orderFixture(options.legacy)
  const state = { failBack: options.failBack ?? false }
  await page.context().route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname
    if (path === '/api/admin/auth/me') return route.fulfill({ json: {
      id: 'staff-reader', email: 'staff@example.test', display_name: 'Equipa', role: 'staff', capabilities: ['orders.read'],
    } })
    if (path === '/api/admin/orders') return route.fulfill({ json: [{
      ...order, customer_name: 'Marta Silva', customer_email: 'marta@example.test', line_count: 2,
    }] })
    if (path === `/api/admin/orders/${orderId}`) return route.fulfill({ json: order })
    if (path.endsWith('/original')) return route.fulfill({
      contentType: 'image/png', body: png, headers: { 'content-disposition': 'attachment; filename=personalizacao-original.png' },
    })
    if (path.includes('/media/')) {
      if (state.failBack && path.includes(backPreview)) return route.fulfill({ status: 404, body: '' })
      const caption = path.includes(frontPreview) ? 'Café é sempre uma boa ideia!' : path.includes(backPreview) ? 'Feito para ti' : ''
      return route.fulfill({ contentType: 'image/svg+xml', body: svg(caption) })
    }
    return route.fulfill({ json: [] })
  })
  await page.goto(`/#orders/${orderId}`)
  await expect(page.getByRole('complementary', { name: `Order ${order.order_number}` })).toBeVisible()
  return state
}

test('shows the purchased product and exact saved personalized faces with downloads for read-only staff', async ({ page }, testInfo) => {
  await fixture(page)
  const items = page.locator('.order-item-card')
  await expect(items).toHaveCount(2)
  const plain = items.first()
  const personalized = items.nth(1)
  await expect(plain.getByText('Produto encomendado', { exact: true })).toBeVisible()
  await expect(plain.locator('.order-saved-image img')).toHaveAttribute('src', `/api/admin/order-product/media/${productPhoto}/detail`)
  await expect(personalized.getByText('Resultado personalizado', { exact: true })).toBeVisible()
  await expect(personalized.locator('.order-saved-image')).toHaveCount(2)
  await expect(personalized.locator('.order-saved-image img').first()).toHaveAttribute('src', `/api/admin/personalization/media/${frontPreview}/detail`)
  await expect(personalized.locator('.order-saved-image img').nth(1)).toHaveAttribute('src', `/api/admin/personalization/media/${backPreview}/detail`)
  await expect(personalized.locator('.order-proof-canvas')).toHaveCount(0)
  await expect(personalized.getByText('Ficheiros originais', { exact: true })).toBeVisible()
  await expect(personalized.locator('.order-customization-specs')).toContainText('Café é sempre uma boa ideia!')
  for (const image of await page.locator('.order-saved-image img').all()) {
    await image.scrollIntoViewIfNeeded()
    await expect(image).toHaveJSProperty('naturalWidth', 600)
    await expect(image).toHaveCSS('object-fit', 'contain')
  }
  const enlarge = personalized.getByRole('link', { name: 'Ampliar imagem · Sweat com capuz · Costas', exact: true })
  const popupEvent = page.waitForEvent('popup')
  await enlarge.click()
  const popup = await popupEvent
  await expect(popup).toHaveURL(new RegExp(`/personalization/media/${backPreview}/detail$`))
  await popup.close()
  await expect(personalized.getByRole('link', { name: 'Descarregar imagem · Sweat com capuz · Costas', exact: true })).toHaveAttribute('href', `/api/admin/personalization/media/${backPreview}/original`)
  await expect(personalized.getByRole('link', { name: 'Descarregar imagem · Sweat com capuz · Costas', exact: true })).toHaveAttribute('download', /KP-2026-000123/)
  await expect(plain.getByRole('link', { name: /Descarregar imagem/ })).toHaveAttribute('download', /KP-2026-000123/)
  expect((await new AxeBuilder({ page }).include('.order-detail').withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']).analyze()).violations).toEqual([])
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
  await personalized.screenshot({ path: testInfo.outputPath('saved-personalization.png') })
  await page.reload()
  await expect(page.locator('.order-saved-image')).toHaveCount(3)
})

test('keeps order details and other images visible when one image fails and allows retrying', async ({ page }) => {
  const state = await fixture(page, { failBack: true })
  const personalized = page.locator('.order-item-card').nth(1)
  const back = personalized.locator('.order-saved-image').nth(1)
  await back.scrollIntoViewIfNeeded()
  await expect(back.getByRole('status')).toContainText('Os detalhes da encomenda continuam guardados.')
  await expect(personalized.locator('.order-customization-specs')).toContainText('Feito para ti')
  await expect(personalized.locator('.order-saved-image img')).toHaveCount(1)
  state.failBack = false
  await back.getByRole('button', { name: 'Tentar novamente' }).click()
  await expect(back.locator('img')).toHaveJSProperty('naturalWidth', 600)
  await expect(back.getByRole('status')).toHaveCount(0)
})

test('keeps earlier personalization proofs and explains when no product photo was saved', async ({ page }) => {
  await fixture(page, { legacy: true })
  const plain = page.locator('.order-item-card').first()
  await expect(plain.locator('.order-saved-image-unavailable')).toContainText('não tem uma fotografia do produto guardada')
  const personalized = page.locator('.order-item-card').nth(1)
  await expect(personalized.locator('.order-production-proof')).toBeVisible()
  await expect(personalized.locator('.order-proof-product')).toHaveCount(2)
  await expect(personalized.locator('.order-customization-specs')).toContainText('Feito para ti')
  await expect(personalized.getByText('Ficheiros originais', { exact: true })).toBeVisible()
  await expect(personalized.locator('.order-saved-images')).toHaveCount(0)
})
