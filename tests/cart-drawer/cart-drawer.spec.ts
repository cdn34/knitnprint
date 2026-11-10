import AxeBuilder from '@axe-core/playwright'
import { expect, test, type Page } from '@playwright/test'
import { createServer, type Server } from 'node:http'
import { randomUUID } from 'node:crypto'

const products = ['Caneca Aurora', 'Sweat personalizada'].map((title, index) => ({
  id: `drawer-product-${index}`, title, slug: index ? 'sweat-personalizada' : 'caneca-aurora', description: 'Uma peça feita para ti.', status: 'active', search_keywords: '', categories: [], personalization_area_price_minor: 100,
  variants: [{ id: `drawer-variant-${index}`, title: index ? 'Rosa · M' : 'Rosa', sku: `DRAWER-${index}`, price_minor: index ? 4000 : 1500, currency: 'EUR', option_values: {}, position: 0, available_quantity: 100, low_stock: false }],
  media: [{ id: `drawer-photo-${index}`, alt_text: title, position: 0, url: '/drawer-product.svg', thumbnail_url: '/drawer-product.svg', card_url: '/drawer-product.svg', detail_url: '/drawer-product.svg' }],
  personalization: { mode: index ? 'photo_text' : 'none', print_areas: [], text_max_characters: 35, text_min_size: 12, text_max_size: 72, allowed_fonts: ['Roboto'], allowed_colors: ['#1f4f78'], views: index ? [{ id: 'front', label: 'Frente', media_id: 'drawer-photo-1', mode: 'photo_text', print_areas: [{ id: 'one', label: 'Área 1', x: 2000, y: 2000, width: 6000, height: 6000, physical_width_cm: 20, physical_height_cm: 20 }] }] : [] },
}))
const feedback = { average_rating: null, total_reviews: 0, rating_counts: [], reviews: [] }
function catalog(path: string) {
  if (path === '/api/products') return products
  if (path.endsWith('/feedback')) return feedback
  return products.find(product => path === `/api/products/${product.slug}`) ?? []
}
let server: Server
test.beforeAll(async () => {
  server = createServer((request, response) => {
    response.setHeader('content-type', 'application/json')
    response.end(JSON.stringify(catalog(request.url!.split('?')[0])))
  })
  await new Promise<void>((resolve, reject) => { server.once('error', reject); server.listen(18084, '127.0.0.1', resolve) })
})
test.afterAll(async () => { await new Promise<void>(resolve => server.close(() => resolve())) })

async function fixture(page: Page) {
  const state = {
    signedIn: true, failAdd: false, failLoad: false, posts: 0, savedPreviews: 0,
    added: [] as Array<{ variant_id: string; customization?: { areas: Array<{ text?: { content: string } }> }; personalization_previews?: unknown[] }>,
    cart: { id: 'drawer-cart', currency: 'EUR', items: [] as Array<Record<string, unknown>>, item_count: 0, subtotal_minor: 0, total_minor: 0, discount_minor: 0, tax_minor: 0, shipping_minor: null, issues: [], checkout_ready: false, expires_at: '2030-01-01T00:00:00Z', delivery: null },
  }
  const filenames = new Map<string, string>()
  await page.addInitScript(() => localStorage.setItem('knitnprint-language', 'pt'))
  await page.route('https://fonts.googleapis.com/**', route => route.fulfill({ contentType: 'text/css', body: '' }))
  await page.route('**/drawer-product.svg', route => route.fulfill({ contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="600" height="600"><rect width="600" height="600" fill="#faf4f0"/><path d="M395 220h30q110 75 0 150h-30" stroke="#bc8b9a" stroke-width="30" fill="none"/><path d="M160 160h240v235q0 65-120 65t-120-65z" fill="#d7adbb"/><ellipse cx="280" cy="160" rx="120" ry="24" fill="#f7e9ed"/></svg>' }))
  await page.route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname
    const unavailable = () => route.fulfill({ status: 503, json: { error: { code: 'cart_unavailable', message: 'Unavailable.' } } })
    const unauthorized = () => route.fulfill({ status: 401, json: { error: { code: 'authentication_required', message: 'Sign in required.' } } })
    if (path === '/api/account/me') return state.signedIn ? route.fulfill({ json: { id: 'drawer-customer', email: 'drawer@example.test', first_name: 'Ana', last_name: 'Silva', phone: '', email_verified: true, addresses: [] } }) : unauthorized()
    if (path === '/api/cart') return !state.signedIn ? unauthorized() : state.failLoad ? unavailable() : route.fulfill({ json: state.cart })
    if (path === '/api/payments/options') return route.fulfill({ json: { stripe: false, manual: true, checkout_suspended: false } })
    if (path === '/api/personalization/uploads') {
      const id = randomUUID(); filenames.set(id, route.request().postDataJSON().filename)
      return route.fulfill({ status: 201, json: { id, upload_url: `http://127.0.0.1:13002/api/test-upload/${id}` } })
    }
    if (path.startsWith('/api/test-upload/')) {
      if (filenames.get(path.split('/').at(-1)!) === 'personalized-preview.png') state.savedPreviews++
      return route.fulfill({ status: 200, body: '' })
    }
    if (path.endsWith('/complete')) return route.fulfill({ json: { id: path.split('/')[4], preview_url: '/drawer-product.svg' } })
    if (path === '/api/cart/items') {
      state.posts++
      if (state.failAdd) return unavailable()
      const input = route.request().postDataJSON()
      state.added.push(input)
      const product = products.find(product => product.variants[0].id === input.variant_id)!
      const variant = product.variants[0]
      const fee = input.customization ? 100 : 0
      state.cart.items.push({ id: randomUUID(), variant_id: variant.id, product_slug: product.slug, product_title: product.title, variant_title: variant.title, sku: variant.sku, quantity: 1, unit_price_minor: variant.price_minor + fee, base_unit_price_minor: variant.price_minor, personalization_unit_price_minor: fee, line_total_minor: variant.price_minor + fee, currency: 'EUR', available: true, available_quantity: 100, image_url: '/drawer-product.svg', customization: input.customization ?? null, customization_media_asset_ids: [] })
      state.cart.item_count = state.cart.items.length
      state.cart.subtotal_minor = state.cart.items.reduce((sum, item) => sum + Number(item.line_total_minor), 0)
      state.cart.total_minor = state.cart.subtotal_minor
      return route.fulfill({ status: 201, json: state.cart })
    }
    return route.fulfill({ json: catalog(path) })
  })
  return state
}

test('returns to products with an accessible cart on the right after adding an ordinary product', async ({ page }, testInfo) => {
  const state = await fixture(page)
  await page.goto('/products/caneca-aurora')
  await page.getByRole('button', { name: 'Adicionar ao carrinho', exact: true }).click()
  await expect(page).toHaveURL(/\/products\/?$/)
  const drawer = page.getByRole('dialog', { name: 'Carrinho' })
  await expect(drawer.locator('.cart-drawer-item')).toHaveCount(1)
  await expect(drawer.getByRole('heading', { name: 'Caneca Aurora' })).toBeVisible()
  await expect(drawer.locator('.cart-drawer-subtotal')).toContainText(/15,00\s*€/)
  await expect(drawer.getByRole('status')).toHaveText('Adicionado ao carrinho')
  await expect(drawer).toBeInViewport({ ratio: 1 })
  const bounds = await drawer.boundingBox()
  expect(Math.abs(bounds!.x + bounds!.width - page.viewportSize()!.width)).toBeLessThanOrEqual(1)
  await expect(drawer.getByRole('link', { name: 'Ver carrinho e finalizar' })).toBeInViewport({ ratio: 1 })
  await expect(drawer.getByRole('button', { name: 'Fechar carrinho' })).toBeFocused()
  for (let index = 0; index < 10; index++) {
    await page.keyboard.press('Tab')
    expect(await page.evaluate(() => Boolean(document.activeElement?.closest('dialog')))).toBe(true)
  }
  expect((await new AxeBuilder({ page }).include('.cart-drawer').analyze()).violations).toEqual([])
  await page.screenshot({ path: testInfo.outputPath('cart-drawer.png') })
  await drawer.getByRole('link', { name: 'Continuar a comprar' }).click()
  await expect(drawer).toHaveCount(0)
  await page.locator('.shop-search input').fill('Sweat')
  await expect(page.locator('.product-card')).toHaveCount(1)
  await page.locator('.cart-icon-button[aria-haspopup="dialog"]').click()
  await expect(drawer.locator('.cart-drawer-item')).toHaveCount(1)
  await page.keyboard.press('Escape')
  await expect(drawer).toHaveCount(0)
  await expect(page.locator('.cart-icon-button[aria-haspopup="dialog"]')).toBeFocused()
  await page.reload()
  await expect(drawer).toHaveCount(0)
  await page.locator('.cart-icon-button[aria-haspopup="dialog"]').click()
  await drawer.getByRole('link', { name: 'Ver carrinho e finalizar' }).click()
  await expect(page).toHaveURL(/\/cart$/)
  await expect(page.locator('.cart-item')).toHaveCount(1)
  expect(state.posts).toBe(1)
})

test('keeps the confirmation and personalization before returning to products with the cart open', async ({ page }) => {
  const state = await fixture(page)
  await page.goto('/products/sweat-personalizada/personalize')
  await page.getByRole('textbox', { name: /^O teu texto/ }).fill('O meu presente')
  await page.getByRole('button', { name: 'Adicionar ao carrinho', exact: true }).click()
  const confirmation = page.getByRole('alertdialog')
  await confirmation.getByRole('button', { name: 'Continuar a editar' }).click()
  expect(state.posts).toBe(0)
  expect(state.savedPreviews).toBe(0)
  await expect(page.getByRole('textbox', { name: /^O teu texto/ })).toHaveValue('O meu presente')
  await page.getByRole('button', { name: 'Adicionar ao carrinho', exact: true }).click()
  await confirmation.getByRole('button', { name: 'Adicionar ao carrinho', exact: true }).click()
  await expect(page).toHaveURL(/\/products\/?$/)
  const drawer = page.getByRole('dialog', { name: 'Carrinho' })
  await expect(drawer.locator('.cart-drawer-personalization')).toContainText('Personalizado')
  await expect(drawer.locator('.cart-drawer-price-breakdown')).toHaveText(/40,00\s*€.*1,00\s*€/)
  await expect(drawer.locator('.cart-drawer-subtotal')).toContainText(/41,00\s*€/)
  expect(state.added[0].customization!.areas[0].text!.content).toBe('O meu presente')
  expect(state.added[0].personalization_previews).toHaveLength(1)
  expect(state.savedPreviews).toBe(1)
  await drawer.getByRole('link', { name: 'Continuar a comprar' }).click()
  await page.locator('.product-card').filter({ hasText: 'Caneca Aurora' }).locator('.product-visual').click()
  await page.getByRole('button', { name: 'Adicionar ao carrinho', exact: true }).click()
  await expect(page).toHaveURL(/\/products\/?$/)
  await expect(drawer.locator('.cart-drawer-item')).toHaveCount(2)
  await expect(drawer.locator('.cart-drawer-subtotal')).toContainText(/56,00\s*€/)
  expect(state.posts).toBe(2)
})

test('does not leave the product on failure and can retry loading without adding it twice', async ({ page }) => {
  const state = await fixture(page)
  state.failAdd = true
  await page.goto('/products/caneca-aurora')
  await page.getByRole('button', { name: 'Adicionar ao carrinho', exact: true }).click()
  await expect(page.locator('.cart-action-status')).toContainText('Não foi possível')
  await expect(page).toHaveURL(/\/products\/caneca-aurora$/)
  await expect(page.locator('.cart-drawer')).toHaveCount(0)
  state.failAdd = false
  state.failLoad = true
  await page.getByRole('button', { name: 'Adicionar ao carrinho', exact: true }).click()
  const drawer = page.getByRole('dialog', { name: 'Carrinho' })
  await expect(drawer.getByRole('alert')).toContainText('temporariamente indisponível')
  state.failLoad = false
  await drawer.getByRole('button', { name: 'Tentar novamente' }).click()
  await expect(drawer.locator('.cart-drawer-item')).toHaveCount(1)
  expect(state.posts).toBe(2)
})

test('handles an empty cart and asks guests to sign in', async ({ page }) => {
  const state = await fixture(page)
  await page.goto('/products')
  await page.locator('.cart-icon-button[aria-haspopup="dialog"]').click()
  const drawer = page.getByRole('dialog', { name: 'Carrinho' })
  await expect(drawer.locator('.cart-drawer-state').getByRole('heading')).toBeVisible()
  await expect(drawer.getByRole('link', { name: 'Ver carrinho e finalizar' })).toHaveCount(0)
  await drawer.getByRole('button', { name: 'Fechar carrinho' }).click()
  state.signedIn = false
  await page.locator('.cart-icon-button[aria-haspopup="dialog"]').click()
  await expect(drawer).toContainText('Inicia sessão para veres o teu carrinho.')
  await expect(drawer.getByRole('link', { name: 'Iniciar sessão' })).toHaveAttribute('href', '/account')
})
