import { expect, test } from '@playwright/test'

const ownerEmail = process.env.E2E_OWNER_EMAIL ?? 'owner@knitprint.local'
const ownerPassword =
  process.env.E2E_OWNER_PASSWORD ?? 'local-development-passphrase'

test('lets an owner create, preview, search, and publish a product', async ({
  page,
}) => {
  const unique = `${Date.now()}-${test.info().retry}`
  const title = `Woven Planter ${unique}`
  const slug = `woven-planter-${unique}`
  const sku = `PLANTER-${unique}`

  await page.goto('/')
  await page.getByLabel('Email address').fill(ownerEmail)
  await page.getByLabel('Password').fill(ownerPassword)
  await page.getByRole('button', { name: 'Sign in' }).click()

  const catalog = page.getByRole('region', { name: 'Products' })
  await expect(catalog).toBeVisible()
  await catalog.getByLabel('Product title').fill(title)
  await catalog.getByLabel('URL slug').fill(slug)
  await catalog
    .getByLabel('Description')
    .fill('A tactile home for printed forms and soft stitches.')
  await catalog.getByLabel('Search keywords').fill(`browsercatalog ${unique}`)
  await catalog.getByLabel('SKU').fill(sku)
  await catalog.getByLabel('Price').fill('42.00')
  await catalog.getByRole('button', { name: 'Create draft' }).click()

  const product = catalog.getByRole('article').filter({ hasText: slug })
  await expect(product).toContainText(title)
  await expect(product).toContainText('draft')
  await expect(
    catalog.getByRole('article', { name: 'Product preview' }),
  ).toContainText(title)

  await catalog.getByLabel('Search products').fill(`browsercatalog ${unique}`)
  await expect(product).toBeVisible()
  await product.getByRole('button', { name: 'Publish' }).click()
  await expect(product).toContainText('active')

  const publicResponse = await page.request.get(`/api/products/${slug}`)
  expect(publicResponse.ok()).toBeTruthy()
  await expect(publicResponse.json()).resolves.toMatchObject({
    title,
    slug,
    status: 'active',
    variants: [{ sku, price_minor: 4200, currency: 'EUR' }],
  })
})
