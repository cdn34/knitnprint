import { expect, test } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import { signInVerifiedCustomer } from '../helpers/customer'

const ownerEmail = process.env.E2E_OWNER_EMAIL ?? 'owner@knitnprint.local'
const ownerPassword = process.env.E2E_OWNER_PASSWORD ?? 'local-development-passphrase'
const shop = process.env.E2E_STOREFRONT_URL ?? 'http://127.0.0.1:3000'

test('saves independent area permissions and enforces them in the editor, preview and cart', async ({ page }, testInfo) => {
  test.setTimeout(90_000)
  const unique = `${Date.now()}-${testInfo.project.name}`
  expect((await page.request.post('/api/admin/auth/login', { data: { email: ownerEmail, password: ownerPassword } })).ok()).toBe(true)
  const created = await page.request.post('/api/admin/products', { data: {
    title: `Area permissions ${unique}`, slug: `area-permissions-${unique}`,
    variants: [{ title: 'Default', price_minor: 4000, currency: 'EUR', available_quantity: 5 }],
  } })
  expect(created.status()).toBe(201)
  const product = await created.json()
  const bytes = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=', 'base64')
  const initiated = await page.request.post('/api/admin/media/uploads', { data: { filename: 'product.png', content_type: 'image/png', byte_size: bytes.length } })
  expect(initiated.status()).toBe(201)
  const media = await initiated.json()
  expect((await page.request.put(media.upload_url, { data: bytes, headers: { 'content-type': 'image/png' } })).ok()).toBe(true)
  expect((await page.request.post(`/api/admin/media/uploads/${media.id}/complete`, { data: { product_id: product.id, alt_text: 'Product for area permissions' } })).ok()).toBe(true)

  async function editProduct() {
    await page.goto('/#products')
    await page.reload()
    await page.getByRole('article').filter({ hasText: product.title }).getByRole('button', { name: 'Edit', exact: true }).click()
  }
  async function saveProduct() {
    const saved = page.waitForResponse(response => new URL(response.url()).pathname === `/api/admin/products/${product.id}` && response.request().method() === 'PUT')
    await page.getByRole('button', { name: 'Save product', exact: true }).click()
    expect((await saved).ok()).toBe(true)
    await expect(page.getByRole('button', { name: 'Save product', exact: true })).toBeEnabled()
  }
  const manager = page.locator('.print-area-manager')
  async function selectArea(number: number) {
    await manager.getByRole('tab', { name: new RegExp(`Área ${number}$`) }).click()
  }
  await editProduct()
  await page.locator('#personalization-mode').selectOption('photo_text')
  await manager.getByRole('button', { name: 'Adicionar área', exact: false }).click()
  await page.getByLabel('Personalização permitida nesta área', { exact: true }).selectOption('text')
  await manager.getByRole('button', { name: 'Adicionar área', exact: false }).click()
  await page.getByLabel('Personalização permitida nesta área', { exact: true }).selectOption('photo')
  expect((await new AxeBuilder({ page }).include('.print-area-content-settings').withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']).analyze()).violations).toEqual([])
  await saveProduct()
  const initial = await (await page.request.get(`/api/admin/products/${product.id}`)).json()
  expect(initial.personalization.views[0].print_areas.map((area: { mode?: string }) => area.mode)).toEqual(['photo_text', 'text', 'photo'])
  expect((await page.request.post(`/api/admin/products/${product.id}/status`, { data: { status: 'active' } })).ok()).toBe(true)
  await signInVerifiedCustomer(page.request, `areas-${unique}@example.test`, 'Area', 'Tester')
  const personalizerUrl = `${shop}/products/${product.slug}/personalize?variant=${product.variants[0].id}`
  await page.goto(personalizerUrl)
  const choices = page.locator('.personalizer-area-switcher')
  await choices.getByRole('button', { name: /Área 3$/ }).click()
  await expect(page.locator('.personalizer-tools input[type=file]')).toHaveCount(1)
  await expect(page.locator('.personalizer-tools textarea')).toHaveCount(0)

  await editProduct()
  await selectArea(3)
  await expect(page.locator('#print-area-mode')).toHaveValue('photo')
  await page.locator('#print-area-mode').selectOption('text')
  await selectArea(1)
  await expect(page.locator('#print-area-mode')).toHaveValue('photo_text')
  await selectArea(2)
  await expect(page.locator('#print-area-mode')).toHaveValue('text')
  await saveProduct()
  await editProduct()
  await selectArea(3)
  await expect(page.locator('#print-area-mode')).toHaveValue('text')
  await manager.screenshot({ path: testInfo.outputPath('admin-area-permissions.png') })

  await page.goto(personalizerUrl)
  await page.locator('.personalizer-tools textarea').fill('Área principal')
  const completed = page.waitForResponse(response => /\/api\/personalization\/uploads\/[^/]+\/complete$/.test(new URL(response.url()).pathname), { timeout: 15000 })
  await page.locator('.personalizer-tools input[type=file]').setInputFiles({ name: 'design.png', mimeType: 'image/png', buffer: bytes })
  expect((await completed).ok()).toBe(true)
  for (const number of [2, 3]) {
    await choices.getByRole('button', { name: new RegExp(`Área ${number}$`) }).click()
    await expect(page.locator('.personalizer-tools input[type=file]')).toHaveCount(0)
    await page.locator('.personalizer-tools textarea').fill(`Texto da área ${number}`)
    await expect(page.locator('.personalizer-print-area').first().locator('.personalizer-photo')).toHaveCount(1)
  }
  await page.locator('.personalizer-layout').screenshot({ path: testInfo.outputPath('customer-area-permissions.png') })
  await page.getByRole('button', { name: 'Preview result', exact: true }).click()
  await expect(page.getByRole('dialog').locator('.personalization-final-photo')).toHaveCount(1)
  await expect(page.getByRole('dialog').locator('.personalization-final-element--text')).toHaveCount(3)
  const added = page.waitForResponse(response => new URL(response.url()).pathname === '/api/cart/items' && response.request().method() === 'POST')
  await page.getByRole('dialog').getByRole('button', { name: 'Add to cart', exact: true }).click()
  const addedResponse = await added
  expect(addedResponse.status()).toBe(201)
  const input = addedResponse.request().postDataJSON()
  expect(input.customization.areas.map((area: { photo?: unknown; text?: unknown }) => [Boolean(area.photo), Boolean(area.text)])).toEqual([[true, true], [false, true], [false, true]])
  await expect(page).toHaveURL(new RegExp(`${shop}/products/?$`))
  await expect(page.getByRole('dialog', { name: 'Cart', exact: true }).locator('.cart-drawer-personalization')).toContainText('Personalized')
  const forged = structuredClone(input)
  forged.customization.areas[1].photo = forged.customization.areas[0].photo
  const rejected = await page.request.post(`${shop}/api/cart/items`, { headers: { 'idempotency-key': `forbidden-area-${unique}` }, data: forged })
  expect(rejected.ok()).toBe(false)
  expect((await rejected.json()).error.code).toBe('invalid_customization')
})
