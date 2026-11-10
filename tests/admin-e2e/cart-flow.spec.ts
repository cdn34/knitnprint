import { expect, test } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import { signInVerifiedCustomer } from '../helpers/customer'

const ownerEmail = process.env.E2E_OWNER_EMAIL ?? 'owner@knitnprint.local'
const ownerPassword = process.env.E2E_OWNER_PASSWORD ?? 'local-development-passphrase'
const shop = process.env.E2E_STOREFRONT_URL ?? 'http://127.0.0.1:3000'

async function fixture(page: import('@playwright/test').Page, personalized = false) {
  const unique = `${Date.now()}-${Math.random().toString(36).slice(2)}`
  expect((await page.request.post('/api/admin/auth/login', { data: { email: ownerEmail, password: ownerPassword } })).ok()).toBe(true)
  const created = await page.request.post('/api/admin/products', { data: {
    title: `Cart flow ${unique}`, slug: `cart-flow-${unique}`,
    variants: [{ title: 'Default', price_minor: personalized ? 4000 : 1500, currency: 'EUR', available_quantity: 5 }],
  } })
  expect(created.status()).toBe(201)
  const product = await created.json()
  if (personalized) {
    await page.goto('/')
    const png = await page.evaluate(() => {
      const canvas = document.createElement('canvas')
      canvas.width = 600; canvas.height = 600
      const context = canvas.getContext('2d')!
      context.fillStyle = '#f5e9ed'; context.fillRect(0, 0, 600, 600)
      return canvas.toDataURL('image/png').split(',')[1]
    })
    const bytes = Buffer.from(png, 'base64')
    const started = await page.request.post('/api/admin/media/uploads', { data: { filename: 'product.png', content_type: 'image/png', byte_size: bytes.length } })
    expect(started.status()).toBe(201)
    const media = await started.json()
    expect((await page.request.put(media.upload_url, { data: bytes, headers: { 'content-type': 'image/png' } })).ok()).toBe(true)
    expect((await page.request.post(`/api/admin/media/uploads/${media.id}/complete`, { data: { product_id: product.id, alt_text: 'Product for personalization flow' } })).ok()).toBe(true)
    expect((await page.request.put(`/api/admin/products/${product.id}`, { data: {
      title: product.title, slug: product.slug, description: '', price_minor: 4000, currency: 'EUR', available_quantity: 5,
      personalization: { ...product.personalization, mode: 'photo_text', views: [{ id: 'front', label: 'Front', mode: 'photo_text', media_id: media.id,
        print_areas: [{ id: 'main', label: 'Area 1', mode: 'photo_text', x: 2000, y: 2000, width: 6000, height: 6000, physical_width_cm: 20, physical_height_cm: 20 }] }] },
    } })).ok()).toBe(true)
  }
  expect((await page.request.post(`/api/admin/products/${product.id}/status`, { data: { status: 'active' } })).ok()).toBe(true)
  await signInVerifiedCustomer(page.request, `cart-flow-${unique}@example.test`, 'Cart', 'Tester')
  return product
}

test('opens a cart on the right over the product catalog after a regular addition and can proceed to checkout', async ({ page }, testInfo) => {
  const product = await fixture(page)
  await page.goto(`${shop}/products/${product.slug}`)
  await page.getByRole('button', { name: 'Add to cart', exact: true }).click()
  await expect(page).toHaveURL(`${shop}/products`)
  const drawer = page.getByRole('dialog', { name: 'Cart', exact: true })
  await expect(drawer.getByRole('heading', { name: product.title, exact: true })).toBeVisible()
  await expect(drawer.locator('.cart-drawer-item')).toHaveCount(1)
  await expect(drawer.locator('.cart-drawer-success')).toHaveText('Added to cart')
  await expect(drawer.locator('.cart-drawer-subtotal')).toContainText(/15[.,]00/)
  await expect(drawer.getByRole('button', { name: 'Close cart' })).toBeFocused()
  await expect.poll(async () => {
    const bounds = await drawer.boundingBox()
    return Math.abs(bounds!.x + bounds!.width - page.viewportSize()!.width)
  }).toBeLessThanOrEqual(1)
  const bounds = await drawer.boundingBox()
  expect(Math.abs(bounds!.x + bounds!.width - page.viewportSize()!.width)).toBeLessThanOrEqual(1)
  expect(bounds!.width).toBeLessThanOrEqual(480)
  await expect(drawer.getByRole('link', { name: 'View cart and checkout' })).toBeInViewport()
  for (let index = 0; index < 8; index++) {
    await page.keyboard.press('Tab')
    expect(await page.evaluate(() => Boolean(document.activeElement?.closest('dialog')))).toBe(true)
  }
  expect((await new AxeBuilder({ page }).include('.cart-drawer').withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']).analyze()).violations).toEqual([])
  await page.screenshot({ path: testInfo.outputPath('cart-over-products.png') })
  await drawer.getByRole('link', { name: 'Continue shopping' }).click()
  await expect(drawer).toHaveCount(0)
  await expect(page).toHaveURL(`${shop}/products`)
  await page.locator('.shop-search input').fill(product.title)
  await expect(page.locator('.product-card')).toHaveCount(1)
  await page.locator('.cart-icon-button[aria-haspopup="dialog"]').click()
  await expect(drawer.locator('.cart-drawer-item')).toHaveCount(1)
  await page.keyboard.press('Escape')
  await expect(drawer).toHaveCount(0)
  await expect(page.locator('.cart-icon-button')).toBeFocused()
  await page.reload()
  await expect(drawer).toHaveCount(0)
  await page.locator('.cart-icon-button[aria-haspopup="dialog"]').click()
  await drawer.getByRole('link', { name: 'View cart and checkout' }).click()
  await expect(page).toHaveURL(`${shop}/cart`)
  await expect(page.locator('.cart-item')).toHaveCount(1)
  await expect(page.getByRole('heading', { name: 'Contact and delivery', exact: true })).toBeVisible()
})

test('returns to the catalog with the preserved design from both personalization add buttons', async ({ page }) => {
  test.setTimeout(90000)
  const product = await fixture(page, true)
  const address = `${shop}/products/${product.slug}/personalize?variant=${product.variants[0].id}`
  const drawer = page.getByRole('dialog', { name: 'Cart', exact: true })
  for (const fromPreview of [false, true]) {
    await page.goto(address)
    await page.locator('.personalizer-tools textarea').fill(fromPreview ? 'From final preview' : 'From editor')
    if (fromPreview) {
      await page.getByRole('button', { name: 'Preview result', exact: true }).click()
      await page.getByRole('dialog').getByRole('button', { name: 'Add to cart', exact: true }).click()
    } else {
      await page.getByRole('button', { name: 'Add to cart', exact: true }).click()
      await page.getByRole('alertdialog').getByRole('button', { name: 'Continue editing' }).click()
      await expect(page.locator('.personalizer-tools textarea')).toHaveValue('From editor')
      await page.getByRole('button', { name: 'Add to cart', exact: true }).click()
    }
    const added = page.waitForResponse(response => new URL(response.url()).pathname === '/api/cart/items' && response.request().method() === 'POST')
    await page.getByRole('alertdialog').getByRole('button', { name: 'Yes, add it', exact: true }).click()
    const response = await added
    expect(response.status()).toBe(201)
    const input = response.request().postDataJSON()
    expect(input.customization.areas[0].text.content).toBe(fromPreview ? 'From final preview' : 'From editor')
    expect(input.personalization_previews).toHaveLength(1)
    await expect(page).toHaveURL(`${shop}/products`)
    await expect(drawer.locator('.cart-drawer-personalization')).toHaveCount(fromPreview ? 2 : 1)
    await expect(drawer.locator('.cart-drawer-personalization').first()).toContainText('Personalized')
    await drawer.getByRole('link', { name: 'Continue shopping' }).click()
  }
  await page.goto(`${shop}/products/${product.slug}`)
  await page.getByRole('button', { name: 'Add to cart', exact: true }).click()
  await expect(page).toHaveURL(`${shop}/products`)
  await expect(drawer.locator('.cart-drawer-item')).toHaveCount(3)
  await expect(drawer.locator('.cart-drawer-personalization')).toHaveCount(2)
})

test('preserves the product on add failure and can retry reading the drawer without adding twice', async ({ page }) => {
  const product = await fixture(page)
  let failAdd = true, failLoad = false, posts = 0
  await page.route('**/api/cart/items', async route => {
    posts++
    if (failAdd) return route.fulfill({ status: 503, json: { error: { code: 'cart_unavailable', message: 'Test outage' } } })
    return route.continue()
  })
  await page.route('**/api/cart', route => failLoad
    ? route.fulfill({ status: 503, json: { error: { code: 'cart_unavailable', message: 'Test outage' } } })
    : route.continue())
  await page.goto(`${shop}/products/${product.slug}`)
  await page.getByRole('button', { name: 'Add to cart', exact: true }).click()
  await expect(page.locator('.cart-action-status')).toContainText('Please try again')
  await expect(page).toHaveURL(`${shop}/products/${product.slug}`)
  await expect(page.getByRole('dialog')).toHaveCount(0)
  failAdd = false; failLoad = true
  await page.getByRole('button', { name: 'Add to cart', exact: true }).click()
  const drawer = page.getByRole('dialog', { name: 'Cart', exact: true })
  await expect(page).toHaveURL(`${shop}/products`)
  await expect(drawer.getByRole('alert')).toContainText('temporarily unavailable')
  failLoad = false
  await drawer.getByRole('button', { name: 'Try again', exact: true }).click()
  await expect(drawer.locator('.cart-drawer-item')).toHaveCount(1)
  expect(posts).toBe(2)
})

test('opens the cart from the header for guests and handles an empty signed-in cart', async ({ page }) => {
  await page.goto(`${shop}/products`)
  await expect(page.locator('.cart-icon-button')).toHaveAttribute('aria-haspopup', 'dialog')
  await page.locator('.cart-icon-button[aria-haspopup="dialog"]').click()
  const drawer = page.getByRole('dialog', { name: 'Cart', exact: true })
  await expect(drawer).toContainText('Sign in to see your cart.')
  await expect(drawer.getByRole('link', { name: 'Sign in', exact: true })).toHaveAttribute('href', '/account')
  await drawer.getByRole('button', { name: 'Close cart' }).click()
  await fixture(page)
  await page.goto(`${shop}/products`)
  await page.locator('.cart-icon-button[aria-haspopup="dialog"]').click()
  await expect(drawer.locator('.cart-drawer-state h3')).toBeVisible()
  await expect(drawer.getByRole('link', { name: 'View cart and checkout' })).toHaveCount(0)
  await drawer.getByRole('link', { name: 'Continue shopping' }).click()
  await expect(page).toHaveURL(`${shop}/products`)
  await expect(drawer).toHaveCount(0)
})
