import { expect, test } from '@playwright/test'
import { signInVerifiedCustomer } from '../helpers/customer'

const ownerEmail = process.env.E2E_OWNER_EMAIL ?? 'owner@knitnprint.local'
const ownerPassword = process.env.E2E_OWNER_PASSWORD ?? 'local-development-passphrase'

test('saves both personalized faces with the order for confirmation emails', async ({ page }) => {
  test.setTimeout(60_000)
  const unique = `${Date.now()}-${test.info().retry}`
  expect((await page.request.post('/api/admin/auth/login', {
    data: { email: ownerEmail, password: ownerPassword },
  })).ok()).toBe(true)
  const created = await page.request.post('/api/admin/products', { data: {
    title: `Email preview ${unique}`, slug: `email-preview-${unique}`,
    variants: [{ title: 'White · M', sku: `EMAIL-${unique}`, price_minor: 2500, currency: 'EUR', available_quantity: 5, option_values: { Size: 'M' } }],
  } })
  expect(created.status()).toBe(201)
  const product = await created.json()
  await page.goto('/')
  const image = await page.evaluate(() => {
    const canvas = document.createElement('canvas')
    canvas.width = 600; canvas.height = 700
    const context = canvas.getContext('2d')!
    context.fillStyle = '#ffffff'; context.fillRect(0, 0, 600, 700)
    return canvas.toDataURL('image/png').split(',')[1]
  })
  const bytes = Buffer.from(image, 'base64')
  const upload = await page.request.post('/api/admin/media/uploads', { data: {
    filename: 'white-product.png', content_type: 'image/png', byte_size: bytes.length,
  } })
  expect(upload.status()).toBe(201)
  const media = await upload.json()
  expect((await page.request.put(media.upload_url, { data: bytes, headers: { 'content-type': 'image/png' } })).ok()).toBe(true)
  expect((await page.request.post(`/api/admin/media/uploads/${media.id}/complete`, { data: { product_id: product.id, alt_text: 'White product for saved preview' } })).ok()).toBe(true)
  const views = ['Front', 'Back'].map((label) => ({
    id: label.toLowerCase(), label, media_id: media.id,
    print_areas: [{ id: `area-${label}`, label: 'Print', x: 2000, y: 2000, width: 6000, height: 6000, physical_width_cm: 20, physical_height_cm: 20 }],
  }))
  expect((await page.request.put(`/api/admin/products/${product.id}`, { data: {
    title: product.title, slug: product.slug, description: '', sku: product.variants[0].sku, price_minor: 2500, currency: 'EUR', available_quantity: 5,
    personalization: { ...product.personalization, mode: 'text', views },
  } })).ok()).toBe(true)
  expect((await page.request.post(`/api/admin/products/${product.id}/status`, { data: { status: 'active' } })).ok()).toBe(true)
  const email = `preview-${unique}@example.test`
  await signInVerifiedCustomer(page.request, email, 'Preview', 'Tester')
  await page.goto(`http://127.0.0.1:3000/products/${product.slug}/personalize`)
  await page.locator('.personalizer-tools textarea').fill('Front design')
  await page.locator('.personalizer-view-switcher').getByRole('tab', { name: /Back/ }).click()
  await page.locator('.personalizer-tools textarea').fill('Back design')
  await page.getByRole('button', { name: 'Preview result', exact: true }).click()
  const preview = page.getByRole('dialog')
  await expect(preview.getByText('Back design', { exact: true })).toBeVisible()
  const added = page.waitForResponse((response) => new URL(response.url()).pathname === '/api/cart/items' && response.request().method() === 'POST')
  await preview.getByRole('button', { name: 'Add to cart', exact: true }).click()
  const response = await added
  expect(response.status()).toBe(201)
  const saved = response.request().postDataJSON().personalization_previews as Array<{ view_id: string; media_id: string }>
  expect(saved.map((view) => view.view_id)).toEqual(['front', 'back'])
  expect(new Set(saved.map((view) => view.media_id)).size).toBe(2)

  const delivery = await page.request.post('http://127.0.0.1:3000/api/cart/delivery', { headers: { 'idempotency-key': `email-delivery-${unique}` }, data: {
    email, first_name: 'Preview', last_name: 'Tester', phone: '',
    address: { recipient_name: 'Preview Tester', line1: '1 Preview Lane', line2: '', city: 'Lisbon', region: '', postal_code: '1000-001', country_code: 'PT', phone: '' },
  } })
  expect(delivery.ok()).toBe(true)
  const orderResponse = await page.request.post('http://127.0.0.1:3000/api/orders', { headers: { 'idempotency-key': `email-checkout-${unique}` }, data: { payment_method: 'manual', locale: 'pt' } })
  expect(orderResponse.status()).toBe(201)
  const order = await orderResponse.json()
  expect(order.lines[0].personalization_previews).toEqual(saved.map((view) => ({ ...view, view_label: view.view_id === 'front' ? 'Front' : 'Back' })))
  expect(order.lines[0].variant_options).toEqual({ Size: 'M' })
  const snapshots = await Promise.all(saved.map(async (view) => {
    const asset = await page.request.get(`/api/admin/personalization/media/${view.media_id}/detail`)
    expect(asset.ok()).toBe(true)
    return asset.body()
  }))
  expect(snapshots[0].equals(snapshots[1])).toBe(false)
})
